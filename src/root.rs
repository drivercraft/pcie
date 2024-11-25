use alloc::vec::Vec;
use pci_types::{ConfigRegionAccess, PciHeader, PciPciBridgeHeader};

use crate::{Chip, Endpoint, Header, PciAddress, PciPciBridge, Unknown};
use core::{hint::spin_loop, ops::Range, ptr::NonNull};

const MAX_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;

pub struct RootComplex<C: Chip> {
    pub(crate) chip: C,
    pub(crate) mmio_base: NonNull<u8>,
}

impl<C> RootComplex<C>
where
    C: Chip,
{
    pub fn new_with_chip(mmio_base: NonNull<u8>, chip: C) -> Self {
        Self { chip, mmio_base }
    }

    pub fn enumerate(&mut self, range: Option<Range<usize>>) -> PciIterator<'_, C> {
        let range = range.unwrap_or_else(|| 0..0x100);

        PciIterator {
            root: self,
            segment: 0,
            bus_max: (range.end - 1) as _,
            function: 0,
            is_mulitple_function: false,
            is_finish: false,
            stack: Vec::new(),
            bus_start: range.start as _,
        }
    }

    pub fn read_config(&self, address: PciAddress, offset: u16) -> u32 {
        unsafe { self.chip.read(self.mmio_base, address, offset) }
    }

    pub fn write_config(&mut self, address: PciAddress, offset: u16, value: u32) {
        unsafe { self.chip.write(self.mmio_base, address, offset, value) }
    }
}

impl<C: Chip> ConfigRegionAccess for RootComplex<C> {
    unsafe fn read(&self, address: pci_types::PciAddress, offset: u16) -> u32 {
        self.read_config(address, offset)
    }

    unsafe fn write(&self, address: pci_types::PciAddress, offset: u16, value: u32) {
        self.chip.write(self.mmio_base, address, offset, value);
    }
}

pub struct PciIterator<'a, C: Chip> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: &'a RootComplex<C>,
    segment: u16,
    stack: Vec<Bridge>,
    bus_start: u8,
    bus_max: u8,
    function: u8,
    is_mulitple_function: bool,
    is_finish: bool,
}

impl<'a, C: Chip> Iterator for PciIterator<'a, C> {
    type Item = Header;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.is_finish {
            if let Some(value) = self.get_current_valid() {
                self.next(match &value {
                    Header::PciPciBridge(bridge) => Some(bridge),
                    _ => None,
                });
                return Some(value);
            } else {
                self.next(None);
            }
        }
        None
    }
}

impl<'a, C: Chip> PciIterator<'a, C> {
    fn get_current_valid(&mut self) -> Option<Header> {
        let address = self.address();

        let pci_header = PciHeader::new(address);
        let access = self.root;
        let (vendor_id, device_id) = pci_header.id(access);
        if vendor_id == 0xffff {
            return None;
        }

        let has_multiple_functions = pci_header.has_multiple_functions(access);
        let status = pci_header.status(access);
        let command = pci_header.command(access);
        self.is_mulitple_function = has_multiple_functions;

        Some(match pci_header.header_type(access) {
            pci_types::HeaderType::Endpoint => {
                let ep = pci_types::EndpointHeader::from_header(pci_header, access).unwrap();

                Header::Endpoint(Endpoint {
                    address,
                    vendor_id,
                    device_id,
                    command,
                    status,
                    has_multiple_functions,
                })
            }
            pci_types::HeaderType::PciPciBridge => {
                let bridge = PciPciBridgeHeader::from_header(pci_header, access).unwrap();
                let want_primary_bus = bridge.primary_bus_number(access);
                // let want_subordinate_bus = bridge.subordinate_bus_number(access);
                let want_secondary_bus = bridge.secondary_bus_number(access);

                let primary_bus = address.bus();
                let secondary_bus = self
                    .stack
                    .last()
                    .map(|p| p.header.subordinate_bus)
                    .unwrap_or_default();

                assert_eq!(want_primary_bus, primary_bus);
                assert_eq!(want_secondary_bus, secondary_bus);

                let subordinate_bus = secondary_bus;

                Header::PciPciBridge(PciPciBridge {
                    address,
                    vendor_id,
                    device_id,
                    command,
                    status,
                    has_multiple_functions,
                    secondary_bus,
                    subordinate_bus,
                    primary_bus,
                })
            }
            pci_types::HeaderType::Unknown(u) => Header::Unknown(Unknown {
                address,
                vendor_id,
                device_id,
                command,
                status,
                has_multiple_functions,
                kind: u,
            }),
            _ => Header::Unknown(Unknown {
                address,
                vendor_id,
                device_id,
                command,
                status,
                has_multiple_functions,
                kind: 2,
            }),
        })
    }

    fn address(&self) -> PciAddress {
        let bus;
        let device;

        match self.stack.last() {
            Some(bridge) => {
                bus = bridge.header.secondary_bus;
                device = bridge.device;
            }
            None => {
                bus = self.bus_start;
                device = 0;
            }
        }
        PciAddress::new(self.segment, bus, device, self.function)
    }

    /// 若进位返回true
    fn is_next_function_max(&mut self) -> bool {
        if self.is_mulitple_function {
            if self.function == MAX_FUNCTION {
                self.function = 0;
                true
            } else {
                self.function += 1;
                false
            }
        } else {
            self.function = 0;
            true
        }
    }

    /// 若进位返回true
    fn next_device_not_ok(&mut self) -> bool {
        if self.stack.last().unwrap().device == MAX_DEVICE {
            if let Some(parent) = self.stack.pop() {
                self.is_finish = parent.header.subordinate_bus == self.bus_max;

                parent.header.sync_bus_number(self.root);
                self.function = 0;
                return true;
            } else {
                self.is_finish = true;
            }
        } else {
            self.stack.last_mut().unwrap().device += 1;
        }
        false
    }

    fn next(&mut self, current_bridge: Option<&PciPciBridge>) {
        if let Some(bridge) = current_bridge {
            for parent in &mut self.stack {
                parent.header.subordinate_bus += 1;
            }

            self.stack.push(Bridge {
                header: bridge.clone(),
                device: 1,
            });
            self.function = 0;
            return;
        }

        if self.is_next_function_max() {
            while self.next_device_not_ok() {
                spin_loop();
            }
        }
    }
}

struct Bridge {
    header: PciPciBridge,
    device: u8,
}
