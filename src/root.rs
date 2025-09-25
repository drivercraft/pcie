use alloc::vec::Vec;
use pci_types::ConfigRegionAccess;

use crate::chip::PcieController;
use crate::config::{self, Endpoint, PciConfigSpace, PciHeaderBase};
use crate::{types, PciAddress, PciSpace32, PciSpace64, SimpleBarAllocator};
use core::{hint::spin_loop, ops::Range};

const MAX_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;

pub struct RootComplex {
    pub(crate) controller: PcieController,
    pub(crate) allocator: Option<SimpleBarAllocator>,
}

impl RootComplex {
    /// Create a RootComplex with optional pre-configured BAR allocation spaces.
    /// If `space32`/`space64` provided, an internal SimpleBarAllocator will be created.
    pub fn new(controller: PcieController) -> Self {
        Self {
            controller,
            allocator: None,
        }
    }

    pub fn new_generic(mmio_base: core::ptr::NonNull<u8>) -> Self {
        let ctrl = PcieController::new(crate::chip::PcieGeneric::new(mmio_base));
        Self::new(ctrl)
    }

    pub fn set_space32(&mut self, space: PciSpace32) {
        let a = self.allocator.get_or_insert_with(Default::default);
        a.set_mem32(space).unwrap();
    }

    pub fn set_space64(&mut self, space: PciSpace64) {
        let a = self.allocator.get_or_insert_with(Default::default);
        a.set_mem64(space).unwrap();
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
    type Item = Endpoint;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.is_finish {
            if let Some(value) = self.get_current_valid() {
                match value {
                    PciConfigSpace::PciPciBridge(pci_pci_bridge) => {
                        self.next(Some(pci_pci_bridge));
                    }
                    PciConfigSpace::Endpoint(ep) => {
                        let item = ep;
                        self.next(None);
                        return Some(item);
                    }
                }
            } else {
                self.next(None);
            }
        }
        None
    }
}

impl PciIterator<'_> {
    fn get_current_valid(&mut self) -> Option<PciConfigSpace> {
        let address = self.address();
        let header_base = PciHeaderBase::new(self.root.controller.clone(), address)?;
        self.is_mulitple_function = header_base.has_multiple_functions();

        match header_base.header_type() {
            pci_types::HeaderType::Endpoint => {
                let allocator = if self.do_allocate {
                    self.root.allocator.as_mut()
                } else {
                    None
                };
                let ep = types::config::Endpoint::new(header_base, allocator);
                Some(PciConfigSpace::Endpoint(ep))
            }
            pci_types::HeaderType::PciPciBridge => {
                let mut bridge = types::config::PciPciBridge::new(header_base);
                let primary_bus = address.bus();
                let secondary_bus;

                if let Some(parent) = self.stack.last_mut() {
                    if parent.bridge.subordinate_bus_number() == self.bus_max {
                        return None;
                    }

                    secondary_bus = parent.bridge.subordinate_bus_number() + 1;
                } else {
                    panic!("no parent");
                }
                let subordinate_bus = secondary_bus;
                bridge.update_bus_number(|mut bus| {
                    bus.primary = primary_bus;
                    bus.secondary = secondary_bus;
                    bus.subordinate = subordinate_bus;
                    bus
                });

                Some(PciConfigSpace::PciPciBridge(bridge))
            }
            pci_types::HeaderType::CardBusBridge => todo!(),
            pci_types::HeaderType::Unknown(_) => todo!(),
            _ => unreachable!(),
        }
    }

    fn address(&self) -> PciAddress {
        let parent = self.stack.last().unwrap();
        let bus = parent.bridge.secondary_bus_number();
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
                    self.is_finish = parent.bridge.subordinate_bus_number() == self.bus_max;

                    // parent.header.sync_bus_number(&self.root);
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

    fn next(&mut self, current_bridge: Option<config::PciPciBridge>) {
        if let Some(bridge) = current_bridge {
            for parent in &mut self.stack {
                // parent.header.subordinate_bus += 1;

                parent.bridge.update_bus_number(|mut bus| {
                    bus.subordinate += 1;
                    bus
                });
            }

            self.stack.push(Bridge { bridge, device: 0 });

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

// impl PciIterator<'_> {
//     fn get_current_valid(&mut self) -> Option<Header> {
//         let address = self.address();

//         let pci_header = PciHeader::new(address);
//         let access = &self.root;
//         let (vendor_id, device_id) = pci_header.id(access);
//         if vendor_id == 0xffff {
//             return None;
//         }

//         let status = pci_header.status(access);
//         let command = pci_header.command(access);
//         let has_multiple_functions = pci_header.has_multiple_functions(access);
//         let (device_revision, base_class, sub_class, interface) =
//             pci_header.revision_and_class(access);

//         self.is_mulitple_function = has_multiple_functions;

//         Some(match pci_header.header_type(&*self.root) {
//             pci_types::HeaderType::Endpoint => {
//                 // Create endpoint header and read current state
//                 let mut ep = {
//                     let access = &*self.root;
//                     pci_types::EndpointHeader::from_header(pci_header, access).unwrap()
//                 };

//                 let mut bar = {
//                     let access = &*self.root;
//                     ep.parse_bar(6, access)
//                 };
//                 let (interrupt_pin, interrupt_line) = {
//                     let access = &*self.root;
//                     ep.interrupt(access)
//                 };
//                 let capability_pointer = {
//                     let access = &*self.root;
//                     ep.capability_pointer(access)
//                 };
//                 let capabilities = {
//                     let access = &*self.root;
//                     ep.capabilities(access).collect::<Vec<_>>()
//                 };

//                 // Allocate BARs if requested and allocator present
//                 if self.do_allocate && self.root.allocator.is_some() {
//                     // Disable IO/MEM before reprogramming BARs
//                     {
//                         let access = &*self.root;
//                         ep.update_command(access, |mut cmd| {
//                             cmd.remove(CommandRegister::IO_ENABLE);
//                             cmd.remove(CommandRegister::MEMORY_ENABLE);
//                             cmd
//                         });
//                     }

//                     match &bar {
//                         crate::BarVec::Memory32(bar_vec) => {
//                             // Compute new values with mutable allocator, then write using immutable access
//                             let new_vals = {
//                                 let a = self.root.allocator.as_mut().unwrap();
//                                 bar_vec
//                                     .iter()
//                                     .map(|old| {
//                                         old.clone().map(|ref b| {
//                                             a.alloc_memory32_with_pref(b.size, b.prefetchable)
//                                                 .unwrap()
//                                         })
//                                     })
//                                     .collect::<alloc::vec::Vec<_>>()
//                             };
//                             let access = &*self.root;
//                             for (i, v) in new_vals.into_iter().enumerate() {
//                                 if let Some(value) = v {
//                                     bar_vec.set(i, value, access).unwrap();
//                                 }
//                             }
//                         }
//                         crate::BarVec::Memory64(bar_vec) => {
//                             let new_vals = {
//                                 let a = self.root.allocator.as_mut().unwrap();
//                                 bar_vec
//                                     .iter()
//                                     .map(|old| {
//                                         old.clone().map(|ref b| {
//                                             if b.address > 0 && b.address < u32::MAX as u64 {
//                                                 a.alloc_memory32_with_pref(
//                                                     b.size as u32,
//                                                     b.prefetchable,
//                                                 )
//                                                 .unwrap()
//                                                     as u64
//                                             } else {
//                                                 a.alloc_memory64_with_pref(b.size, b.prefetchable)
//                                                     .unwrap()
//                                             }
//                                         })
//                                     })
//                                     .collect::<alloc::vec::Vec<_>>()
//                             };
//                             let access = &*self.root;
//                             for (i, v) in new_vals.into_iter().enumerate() {
//                                 if let Some(value) = v {
//                                     bar_vec
//                                         .set(i, value, access)
//                                         .inspect_err(|e| error!("{e:?}"))
//                                         .unwrap();
//                                 }
//                             }
//                         }
//                         crate::BarVec::Io(_bar_vec_t) => {}
//                     }

//                     // Reload BARs after programming
//                     let access = &*self.root;
//                     bar = ep.parse_bar(6, access);
//                 }

//                 Header::Endpoint(Endpoint {
//                     address,
//                     vendor_id,
//                     device_id,
//                     command,
//                     status,
//                     has_multiple_functions,
//                     bar,
//                     device_revision,
//                     base_class,
//                     sub_class,
//                     interface,
//                     interrupt_pin,
//                     interrupt_line,
//                     capability_pointer,
//                     capabilities,
//                 })
//             }
//             pci_types::HeaderType::PciPciBridge => {
//                 // let bridge = PciPciBridgeHeader::from_header(pci_header, access).unwrap();
//                 // let want_primary_bus = bridge.primary_bus_number(access);
//                 // let want_secondary_bus = bridge.secondary_bus_number(access);

//                 let primary_bus = address.bus();
//                 let secondary_bus;

//                 if let Some(parent) = self.stack.last_mut() {
//                     if parent.header.subordinate_bus == self.bus_max {
//                         return None;
//                     }

//                     secondary_bus = parent.header.subordinate_bus + 1;
//                 } else {
//                     panic!("no parent");
//                 }
//                 let subordinate_bus = secondary_bus;

//                 Header::PciPciBridge(PciPciBridge {
//                     address,
//                     vendor_id,
//                     device_id,
//                     command,
//                     status,
//                     has_multiple_functions,
//                     secondary_bus,
//                     subordinate_bus,
//                     primary_bus,
//                     device_revision,
//                     base_class,
//                     sub_class,
//                     interface,
//                 })
//             }
//             pci_types::HeaderType::Unknown(u) => Header::Unknown(Unknown {
//                 address,
//                 vendor_id,
//                 device_id,
//                 command,
//                 status,
//                 has_multiple_functions,
//                 kind: u,
//                 device_revision,
//                 base_class,
//                 sub_class,
//                 interface,
//             }),
//             _ => Header::CardBusBridge(CardBusBridge {
//                 address,
//                 vendor_id,
//                 device_id,
//                 command,
//                 status,
//                 has_multiple_functions,
//                 device_revision,
//                 base_class,
//                 sub_class,
//                 interface,
//             }),
//         })
//     }

//     fn address(&self) -> PciAddress {
//         let parent = self.stack.last().unwrap();
//         let bus = parent.header.secondary_bus;
//         let device = parent.device;

//         PciAddress::new(self.segment, bus, device, self.function)
//     }

//     /// 若进位返回true
//     fn is_next_function_max(&mut self) -> bool {
//         if self.is_mulitple_function {
//             if self.function == MAX_FUNCTION {
//                 self.function = 0;
//                 true
//             } else {
//                 self.function += 1;
//                 false
//             }
//         } else {
//             self.function = 0;
//             true
//         }
//     }

//     /// 若进位返回true
//     fn next_device_not_ok(&mut self) -> bool {
//         if let Some(parent) = self.stack.last_mut() {
//             if parent.device == MAX_DEVICE {
//                 if let Some(parent) = self.stack.pop() {
//                     self.is_finish = parent.header.subordinate_bus == self.bus_max;

//                     parent.header.sync_bus_number(&self.root);
//                     self.function = 0;
//                     return true;
//                 } else {
//                     self.is_finish = true;
//                 }
//             } else {
//                 parent.device += 1;
//             }
//         } else {
//             self.is_finish = true;
//         }

//         false
//     }

//     fn next(&mut self, current_bridge: Option<&PciPciBridge>) {
//         if let Some(bridge) = current_bridge {
//             for parent in &mut self.stack {
//                 parent.header.subordinate_bus += 1;
//             }

//             self.stack.push(Bridge {
//                 header: bridge.clone(),
//                 device: 0,
//             });

//             self.function = 0;
//             return;
//         }

//         if self.is_next_function_max() {
//             while self.next_device_not_ok() {
//                 spin_loop();
//             }
//         }
//     }
// }

struct Bridge {
    bridge: config::PciPciBridge,
    device: u8,
}

impl Bridge {
    fn root(bus_start: u8) -> Self {
        Self {
            bridge: config::PciPciBridge::root(),
            device: bus_start,
        }
    }
}
