use core::{fmt::Debug, ops::Deref};

use pci_types::{Bar, CommandRegister, ConfigRegionAccess, EndpointHeader, PciAddress};

use crate::{BarHeader, BarVec, SimpleBarAllocator};

pub struct Endpoint {
    base: super::PciHeaderBase,
    header: EndpointHeader,
}

impl Endpoint {
    pub(crate) fn new(
        base: super::PciHeaderBase,
        bar_allocator: Option<&mut SimpleBarAllocator>,
    ) -> Self {
        let header = EndpointHeader::from_header(base.header(), &base.root)
            .expect("EndpointHeader::from_header failed");
        let mut s = Self { base, header };
        if let Some(alloc) = bar_allocator {
            s.realloc_bar(alloc).unwrap();
        }
        s
    }

    pub fn bars(&self) -> BarVec {
        self.header.parse_bar(6, &self.base.root)
    }

    fn realloc_bar(
        &mut self,
        allocator: &mut SimpleBarAllocator,
    ) -> Result<(), pci_types::BarWriteError> {
        // Disable IO/MEM before reprogramming BARs
        self.base.update_command(|mut cmd| {
            cmd.remove(CommandRegister::IO_ENABLE);
            cmd.remove(CommandRegister::MEMORY_ENABLE);
            cmd
        });
        let bar = self.bars();

        match &bar {
            crate::BarVec::Memory32(bar_vec) => {
                // Compute new values with mutable allocator, then write using immutable access
                let new_vals = {
                    bar_vec
                        .iter()
                        .map(|old| {
                            old.clone().map(|ref b| {
                                allocator
                                    .alloc_memory32_with_pref(b.size, b.prefetchable)
                                    .unwrap()
                            })
                        })
                        .collect::<alloc::vec::Vec<_>>()
                };
                for (i, v) in new_vals.into_iter().enumerate() {
                    if let Some(value) = v {
                        bar_vec.set(i, value, &self.base.root).unwrap();
                    }
                }
            }
            crate::BarVec::Memory64(bar_vec) => {
                let new_vals = {
                    bar_vec
                        .iter()
                        .map(|old| {
                            old.clone().map(|ref b| {
                                if b.address > 0 && b.address < u32::MAX as u64 {
                                    allocator
                                        .alloc_memory32_with_pref(b.size as u32, b.prefetchable)
                                        .unwrap() as u64
                                } else {
                                    allocator
                                        .alloc_memory64_with_pref(b.size, b.prefetchable)
                                        .unwrap()
                                }
                            })
                        })
                        .collect::<alloc::vec::Vec<_>>()
                };
                for (i, v) in new_vals.into_iter().enumerate() {
                    if let Some(value) = v {
                        bar_vec
                            .set(i, value, &self.base.root)
                            .inspect_err(|e| error!("{e:?}"))
                            .unwrap();
                    }
                }
            }
            crate::BarVec::Io(_bar_vec_t) => {
                unimplemented!("IO BARs are not supported");
            }
        }

        Ok(())
    }
}

impl Deref for Endpoint {
    type Target = super::PciHeaderBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl BarHeader for EndpointHeader {
    fn read_bar<A: ConfigRegionAccess>(&self, slot: usize, access: &A) -> Option<Bar> {
        self.bar(slot as u8, access)
    }

    fn address(&self) -> PciAddress {
        self.header().address()
    }

    fn header_type(&self) -> pci_types::HeaderType {
        pci_types::HeaderType::Endpoint
    }
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Endpoint")
            .field("base", &self.base)
            .field("bars", &self.bars())
            .finish()
    }
}
