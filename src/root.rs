use core::{
    fmt::Display,
    sync::atomic::{fence, Ordering},
};

use crate::{
    types::{device_type::DeviceType, *},
    Chip, PciDevice, PciDeviceKind,
};
use alloc::vec::Vec;
use log::*;

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
            bus_iter: 0,
            subordinate: 0,
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
    bus_iter: u8,
    subordinate: u8,
}

impl<C: Chip> BusDeviceIterator<C> {
    fn access(&self) -> &C {
        &self.root.chip
    }

    fn current(&self) -> PciAddress {
        PciAddress::new(self.segment, self.bus, self.device, self.function)
    }
}

impl<C: Chip> Iterator for BusDeviceIterator<C> {
    type Item = PciDevice<C>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.function > MAX_FUNCTION {
                self.function = 0;
                self.device += 1;
            }
            if self.device > MAX_DEVICE {
                if let Some(parent) = self.stack.pop() {
                    parent.set_subordinate_bus_number(self.bus_iter, self.access());
                    self.bus = parent.address().bus();
                    self.device = parent.address().device() + 1;
                    self.function = 0;

                    trace!(
                        "{:?} Bridge set primary bus: {}, secondary bus: {}, subordinate bus: {}",
                        parent.address(),
                        parent.primary_bus_number(self.access()),
                        parent.secondary_bus_number(self.access()),
                        parent.subordinate_bus_number(self.access()),
                    );

                    continue;
                } else {
                    return None;
                }
            }
            let current = self.current();
            let header = PciHeader::new(current);

            let (vendor_id, device_id) = header.id(self.access());

            if vendor_id == 0xffff {
                if current.function() == 0 {
                    self.device += 1;
                } else {
                    self.function += 1;
                }
                continue;
            }
            let device = PciDevice::new(self.root.chip.clone(), &header);
            let multi = header.has_multiple_functions(self.access());
            match device.kind() {
                PciDeviceKind::PciPciBridge(bridge) => {
                    bridge
                        .header
                        .set_primary_bus_number(self.bus, self.access());
                    self.bus_iter += 1;
                    self.bus = self.bus_iter;
                    bridge
                        .header
                        .set_secondary_bus_number(self.bus, self.access());
                    bridge
                        .header
                        .set_subordinate_bus_number(0xff, self.access());

                    self.stack
                        .push(PciPciBridgeHeader::from_header(header, self.access()).unwrap());
                    self.device = 0;
                    self.function = 0;
                }
                _ => {
                    if current.function() == 0 && !multi {
                        self.device += 1;
                    } else {
                        self.function += 1;
                    }
                }
            }
            return Some(device);
        }
    }
}
