use core::{
    fmt::Display,
    sync::atomic::{fence, Ordering},
};

use crate::{
    device_type::DeviceType, Bar, Chip, EndpointHeader, HeaderType, PciAddress, PciHeader,
    PciPciBridgeHeader, PciPciBridgeHeaderWrite,
};

use alloc::vec::Vec;
use log::{debug, trace};

const MAX_BUS: u8 = 255;
const MAX_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;
const MAX_BARS: u8 = 6;

/// The root complex of a PCI bus.
#[derive(Clone)]
pub struct RootComplex<C: Chip> {
    chip: C,
}

impl<C: Chip> RootComplex<C> {
    pub fn new(mmio_base: usize) -> Self {
        Self {
            chip: C::new(mmio_base),
        }
    }

    /// Enumerates PCI devices on the given bus.
    pub fn enumerate(&self) -> BusDeviceIterator<C> {
        // Safe because the BusDeviceIterator only reads read-only fields.
        BusDeviceIterator {
            root: self.clone(),
            segment: 0,
            bus: 0,
            device: 0,
            function: 0,
            stack: Vec::new(),
            is_search_bridge: true,
        }
    }
}

#[derive(Debug)]
pub struct FunctionInfo {
    pub addr: PciAddress,
    /// The PCI vendor ID.
    pub vendor_id: u16,
    /// The PCI device ID.
    pub device_id: u16,
    /// The PCI class.
    pub class: u8,
    /// The PCI subclass.
    pub subclass: u8,
    /// The PCI programming interface byte.
    pub prog_if: u8,
    /// The PCI revision ID.
    pub revision: u8,
    /// The type of PCI device.
    pub header_type: HeaderType,
}

impl FunctionInfo {
    pub fn device_type(&self) -> DeviceType {
        DeviceType::from((self.class, self.subclass))
    }
}

impl Display for FunctionInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} {:#X}:{:#X} {:?}:{:?} ",
            self.addr,
            self.vendor_id,
            self.device_id,
            self.header_type,
            self.device_type()
        )
    }
}

pub struct BusDeviceIterator<C: Chip> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: RootComplex<C>,
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
    stack: Vec<PciPciBridgeHeader>,
    is_search_bridge: bool,
}

impl<C: Chip> BusDeviceIterator<C> {
    fn access(&self) -> &C {
        &self.root.chip
    }

    fn current(&self) -> PciAddress {
        PciAddress::new(self.segment, self.bus, self.device, self.function)
    }

    fn handle_ep(&mut self, header: PciHeader) {
        let ep = EndpointHeader::from_header(header, self.access()).unwrap();
        let mut slot = 0;
        while slot < MAX_BARS {
            if let Some(bar) = ep.bar(slot, self.access()) {
                match bar {
                    Bar::Memory32 {
                        address,
                        size,
                        prefetchable,
                    } => {
                        debug!(
                            "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                            slot,
                            address,
                            address + size,
                            " 32bit",
                            if prefetchable { " pref" } else { "" },
                        );
                    }
                    Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    } => {
                        debug!(
                            "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                            slot,
                            address,
                            address + size,
                            " 64bit",
                            if prefetchable { " pref" } else { "" },
                        );
                        slot += 1;
                    }
                    Bar::Io { port } => debug!("  BAR {}: IO  port: {:X}", slot, port),
                }
            }
            fence(Ordering::Release);
            slot += 1;
        }
    }
}

impl<C: Chip> Iterator for BusDeviceIterator<C> {
    type Item = FunctionInfo;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.function > MAX_FUNCTION {
                self.function = 0;
                self.device += 1;
            }
            if self.device > MAX_DEVICE {
                if self.is_search_bridge {
                    self.device = 0;
                } else {
                    if let Some(parent) = self.stack.pop() {
                        let sub = self.bus;
                        self.bus = parent.address().bus();
                        self.device = parent.address().device() + 1;
                        self.function = 0;
                        parent.set_subordinate_bus_number(sub, self.access());
                        trace!("back to {:?}", parent.address());
                    } else {
                        debug!("none!");
                        return None;
                    }
                }
                self.is_search_bridge ^= self.is_search_bridge;
            }
            let current = self.current();
            let header = PciHeader::new(current);

            let (vendor_id, device_id) = header.id(self.access());
            trace!(
                "addr: {:#?} vid {:#X}, did {:#X}",
                current,
                vendor_id,
                device_id
            );
            if vendor_id == 0xffff {
                if current.function() == 0 {
                    self.device += 1;
                } else {
                    self.function += 1;
                }
                continue;
            }

            let header_type = header.header_type(self.access());

            let multi = header.has_multiple_functions(self.access());
            let (revision, class, subclass, interface) = header.revision_and_class(self.access());
            let info = FunctionInfo {
                addr: current,
                vendor_id,
                device_id,
                class,
                subclass,
                prog_if: interface,
                revision,
                header_type,
            };
            trace!("header_type:{:?}", header_type);
            match header_type {
                HeaderType::PciPciBridge => {
                    let bridge = PciPciBridgeHeader::from_header(header, self.access()).unwrap();

                    bridge.set_primary_bus_number(self.bus, self.access());
                    bridge.set_secondary_bus_number((self.bus + 1) as _, self.access());
                    bridge.set_subordinate_bus_number(0xff, self.access());

                    if self.is_search_bridge {
                        self.device += 1;
                        self.function = 0;
                    } else {
                        self.bus += 1;
                        self.device = 0;
                        self.function = 0;
                        self.stack.push(bridge);
                    }
                }
                HeaderType::Endpoint => {
                    if current.function() == 0 && !multi {
                        self.device += 1;
                    } else {
                        self.function += 1;
                    }
                    if self.is_search_bridge {
                        continue;
                    }

                    self.handle_ep(header);
                }
                _ => {
                    debug!("no_header");
                    if current.function() == 0 && !multi {
                        self.device += 1;
                    } else {
                        self.function += 1;
                    }
                    continue;
                }
            }
            return Some(info);
        }
    }
}
