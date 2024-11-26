use alloc::vec::Vec;
use log::error;
use pci_types::{CommandRegister, ConfigRegionAccess, PciHeader, StatusRegister};

use crate::{
    BarAllocator, BarHeader, CardBusBridge, Chip, Endpoint, Header, PciAddress, PciPciBridge,
    Unknown,
};
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

    pub fn enumerate<A: BarAllocator>(
        &mut self,
        range: Option<Range<usize>>,
        bar_alloc: Option<A>,
    ) -> PciIterator<'_, C, A> {
        let range = range.unwrap_or_else(|| 0..0x100);

        PciIterator {
            root: self,
            allocator: bar_alloc,
            segment: 0,
            bus_max: (range.end - 1) as _,
            function: 0,
            is_mulitple_function: false,
            is_finish: false,
            stack: alloc::vec![Bridge::root(range.start as _)],
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

pub struct PciIterator<'a, C: Chip, A: BarAllocator> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: &'a RootComplex<C>,
    allocator: Option<A>,
    segment: u16,
    stack: Vec<Bridge>,
    bus_max: u8,
    function: u8,
    is_mulitple_function: bool,
    is_finish: bool,
}

impl<C: Chip, A: BarAllocator> Iterator for PciIterator<'_, C, A> {
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

impl<C: Chip, A: BarAllocator> PciIterator<'_, C, A> {
    fn get_current_valid(&mut self) -> Option<Header> {
        let address = self.address();

        let pci_header = PciHeader::new(address);
        let access = self.root;
        let (vendor_id, device_id) = pci_header.id(access);
        if vendor_id == 0xffff {
            return None;
        }

        let status = pci_header.status(access);
        let command = pci_header.command(access);
        let has_multiple_functions = pci_header.has_multiple_functions(access);
        let (device_revision, base_class, sub_class, interface) =
            pci_header.revision_and_class(access);

        self.is_mulitple_function = has_multiple_functions;

        Some(match pci_header.header_type(access) {
            pci_types::HeaderType::Endpoint => {
                let access = self.root;

                let ep = pci_types::EndpointHeader::from_header(pci_header, access).unwrap();

                let mut bar = ep.parse_bar(6, access);

                if let Some(a) = &mut self.allocator {
                    match &bar {
                        crate::BarVec::Memory32(bar_vec) => {
                            let new_bar_vec = bar_vec
                                .iter()
                                .map(|old| {
                                    old.clone().map(|ref b| a.alloc_memory32(b.size).unwrap())
                                })
                                .collect::<Vec<_>>();

                            for (i, bar) in new_bar_vec.into_iter().enumerate() {
                                if let Some(value) = bar {
                                    bar_vec.set(i, value, access).unwrap();
                                }
                            }
                        }
                        crate::BarVec::Memory64(bar_vec) => {
                            let new_bar_vec = bar_vec
                                .iter()
                                .map(|old| {
                                    old.clone().map(|ref b| a.alloc_memory64(b.size).unwrap())
                                })
                                .collect::<Vec<_>>();

                            for (i, bar) in new_bar_vec.into_iter().enumerate() {
                                if let Some(value) = bar {
                                    bar_vec
                                        .set(i, value, access)
                                        .inspect_err(|e| error!("{:?}", e))
                                        .unwrap();
                                }
                            }
                        }
                        crate::BarVec::Io(_bar_vec_t) => {}
                    }

                    bar = ep.parse_bar(6, access);
                }

                Header::Endpoint(Endpoint {
                    address,
                    vendor_id,
                    device_id,
                    command,
                    status,
                    has_multiple_functions,
                    bar,
                    device_revision,
                    base_class,
                    sub_class,
                    interface,
                })
            }
            pci_types::HeaderType::PciPciBridge => {
                // let bridge = PciPciBridgeHeader::from_header(pci_header, access).unwrap();
                // let want_primary_bus = bridge.primary_bus_number(access);
                // let want_secondary_bus = bridge.secondary_bus_number(access);

                let primary_bus = address.bus();
                let secondary_bus;

                if let Some(parent) = self.stack.last_mut() {
                    if parent.header.subordinate_bus == self.bus_max {
                        return None;
                    }

                    secondary_bus = parent.header.subordinate_bus + 1;
                } else {
                    panic!("no parent");
                }
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
                    device_revision,
                    base_class,
                    sub_class,
                    interface,
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
                device_revision,
                base_class,
                sub_class,
                interface,
            }),
            _ => Header::CardBusBridge(CardBusBridge {
                address,
                vendor_id,
                device_id,
                command,
                status,
                has_multiple_functions,
                device_revision,
                base_class,
                sub_class,
                interface,
            }),
        })
    }

    fn address(&self) -> PciAddress {
        let parent = self.stack.last().unwrap();
        let bus = parent.header.secondary_bus;
        let device = parent.device;

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
        if let Some(parent) = self.stack.last_mut() {
            if parent.device == MAX_DEVICE {
                if let Some(parent) = self.stack.pop() {
                    self.is_finish = parent.header.subordinate_bus == self.bus_max;

                    parent.header.sync_bus_number(self.root);
                    self.function = 0;
                    return true;
                } else {
                    self.is_finish = true;
                }
            } else {
                parent.device += 1;
            }
        } else {
            self.is_finish = true;
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
                device: 0,
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

impl Bridge {
    fn root(bus_start: u8) -> Self {
        Bridge {
            header: PciPciBridge {
                address: PciAddress::new(0, 0, 0, 0),
                vendor_id: 0,
                device_id: 0,
                command: CommandRegister::empty(),
                status: StatusRegister::new(0),
                has_multiple_functions: true,
                primary_bus: bus_start,
                secondary_bus: bus_start,
                subordinate_bus: bus_start,
                device_revision: 0,
                base_class: 0,
                sub_class: 0,
                interface: 0,
            },
            device: 0,
        }
    }
}
