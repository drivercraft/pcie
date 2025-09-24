use alloc::vec::Vec;
use log::error;
use pci_types::{CommandRegister, ConfigRegionAccess, PciHeader, StatusRegister};

use crate::chip::PcieController;
use crate::{
    BarHeader, CardBusBridge, Endpoint, Header, PciAddress, PciPciBridge, PciSpace32, PciSpace64,
    SimpleBarAllocator, Unknown,
};
use core::{fmt::Display, hint::spin_loop, ops::Range};

const MAX_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;

pub struct RootComplex {
    pub(crate) controller: PcieController,
    pub(crate) allocator: Option<SimpleBarAllocator>,
}

impl RootComplex {
    /// Create a RootComplex with optional pre-configured BAR allocation spaces.
    /// If `space32`/`space64` provided, an internal SimpleBarAllocator will be created.
    pub fn new(
        controller: PcieController,
        space32: Option<PciSpace32>,
        space64: Option<PciSpace64>,
    ) -> Self {
        let mut allocator = None;
        if space32.is_some() || space64.is_some() {
            let mut a = SimpleBarAllocator::default();
            if let Some(s32) = space32 {
                a.set_mem32(s32.address, s32.size);
            }
            if let Some(s64) = space64 {
                a.set_mem64(s64.address, s64.size);
            }
            allocator = Some(a);
        }
        Self {
            controller,
            allocator,
        }
    }

    pub fn new_generic(
        mmio_base: core::ptr::NonNull<u8>,
        space32: Option<PciSpace32>,
        space64: Option<PciSpace64>,
    ) -> Self {
        let ctrl = PcieController::new(crate::chip::PcieGeneric::new(mmio_base));
        Self::new(ctrl, space32, space64)
    }

    /// Set/replace the internal BAR allocator.
    pub fn set_allocator(&mut self, allocator: SimpleBarAllocator) {
        self.allocator = Some(allocator);
    }

    fn __enumerate(&mut self, range: Option<Range<usize>>, do_allocate: bool) -> PciIterator<'_> {
        let range = range.unwrap_or_else(|| 0..0x100);

        PciIterator {
            root: self,
            do_allocate,
            segment: 0,
            bus_max: (range.end - 1) as _,
            function: 0,
            is_mulitple_function: false,
            is_finish: false,
            stack: alloc::vec![Bridge::root(range.start as _)],
        }
    }

    /// enumerate all devices and allocate bars.
    pub fn enumerate(&mut self, range: Option<Range<usize>>) -> PciIterator<'_> {
        self.__enumerate(range, true)
    }

    /// enumerate all devices without modify bar.
    pub fn enumerate_keep_bar(&mut self, range: Option<Range<usize>>) -> PciIterator<'_> {
        self.__enumerate(range, false)
    }

    pub fn read_config(&self, address: PciAddress, offset: u16) -> u32 {
        // PcieController internally manages mutability; see its UnsafeCell usage
        unsafe { self.controller.read(address, offset) }
    }

    pub fn write_config(&mut self, address: PciAddress, offset: u16, value: u32) {
        unsafe { self.controller.write(address, offset, value) }
    }
}

impl ConfigRegionAccess for RootComplex {
    unsafe fn read(&self, address: pci_types::PciAddress, offset: u16) -> u32 {
        self.read_config(address, offset)
    }

    unsafe fn write(&self, address: pci_types::PciAddress, offset: u16, value: u32) {
        self.controller.write(address, offset, value);
    }
}

impl ConfigRegionAccess for &mut RootComplex {
    unsafe fn read(&self, address: pci_types::PciAddress, offset: u16) -> u32 {
        self.read_config(address, offset)
    }

    unsafe fn write(&self, address: pci_types::PciAddress, offset: u16, value: u32) {
        self.controller.write(address, offset, value);
    }
}

pub struct PciIterator<'a> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: &'a mut RootComplex,
    do_allocate: bool,
    segment: u16,
    stack: Vec<Bridge>,
    bus_max: u8,
    function: u8,
    is_mulitple_function: bool,
    is_finish: bool,
}

impl<'a> Iterator for PciIterator<'a> {
    type Item = EnumElem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.is_finish {
            if let Some(value) = self.get_current_valid() {
                self.next(match &value {
                    Header::PciPciBridge(bridge) => Some(bridge),
                    _ => None,
                });
                return Some(EnumElem {
                    root: unsafe { &mut *(self.root as *mut RootComplex) },
                    header: value,
                });
            } else {
                self.next(None);
            }
        }
        None
    }
}

pub struct EnumElem<'a> {
    pub root: &'a mut RootComplex,
    pub header: Header,
}

impl Display for EnumElem<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.header)
    }
}

impl PciIterator<'_> {
    fn get_current_valid(&mut self) -> Option<Header> {
        let address = self.address();

        let pci_header = PciHeader::new(address);
        let access = &self.root;
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

        Some(match pci_header.header_type(&*self.root) {
            pci_types::HeaderType::Endpoint => {
                // Create endpoint header and read current state
                let mut ep = {
                    let access = &*self.root;
                    pci_types::EndpointHeader::from_header(pci_header, access).unwrap()
                };

                let mut bar = {
                    let access = &*self.root;
                    ep.parse_bar(6, access)
                };
                let (interrupt_pin, interrupt_line) = {
                    let access = &*self.root;
                    ep.interrupt(access)
                };
                let capability_pointer = {
                    let access = &*self.root;
                    ep.capability_pointer(access)
                };
                let capabilities = {
                    let access = &*self.root;
                    ep.capabilities(access).collect::<Vec<_>>()
                };

                // Allocate BARs if requested and allocator present
                if self.do_allocate && self.root.allocator.is_some() {
                    // Disable IO/MEM before reprogramming BARs
                    {
                        let access = &*self.root;
                        ep.update_command(access, |mut cmd| {
                            cmd.remove(CommandRegister::IO_ENABLE);
                            cmd.remove(CommandRegister::MEMORY_ENABLE);
                            cmd
                        });
                    }

                    match &bar {
                        crate::BarVec::Memory32(bar_vec) => {
                            // Compute new values with mutable allocator, then write using immutable access
                            let new_vals = {
                                let a = self.root.allocator.as_mut().unwrap();
                                bar_vec
                                    .iter()
                                    .map(|old| {
                                        old.clone().map(|ref b| a.alloc_memory32(b.size).unwrap())
                                    })
                                    .collect::<alloc::vec::Vec<_>>()
                            };
                            let access = &*self.root;
                            for (i, v) in new_vals.into_iter().enumerate() {
                                if let Some(value) = v {
                                    bar_vec.set(i, value, access).unwrap();
                                }
                            }
                        }
                        crate::BarVec::Memory64(bar_vec) => {
                            let new_vals = {
                                let a = self.root.allocator.as_mut().unwrap();
                                bar_vec
                                    .iter()
                                    .map(|old| {
                                        old.clone().map(|ref b| {
                                            if b.address > 0 && b.address < u32::MAX as u64 {
                                                a.alloc_memory32(b.size as u32).unwrap() as u64
                                            } else {
                                                a.alloc_memory64(b.size).unwrap()
                                            }
                                        })
                                    })
                                    .collect::<alloc::vec::Vec<_>>()
                            };
                            let access = &*self.root;
                            for (i, v) in new_vals.into_iter().enumerate() {
                                if let Some(value) = v {
                                    bar_vec
                                        .set(i, value, access)
                                        .inspect_err(|e| error!("{e:?}"))
                                        .unwrap();
                                }
                            }
                        }
                        crate::BarVec::Io(_bar_vec_t) => {}
                    }

                    // Reload BARs after programming
                    let access = &*self.root;
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
                    interrupt_pin,
                    interrupt_line,
                    capability_pointer,
                    capabilities,
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

                    parent.header.sync_bus_number(&self.root);
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
