use core::{fmt::Debug, ops::Index};

use alloc::vec::Vec;
use log::debug;
use pci_types::{Bar, BarWriteError, EndpointHeader, HeaderType, PciAddress, PciHeader};

use crate::{Chip, RootComplex};

#[derive(Debug, Clone)]
pub enum BarVec {
    Memory32(BarVecT<Bar32>),
    Memory64(BarVecT<Bar64>),
    Io(BarVecT<BarIO>),
}

#[derive(Clone)]
pub struct Bar64 {
    pub address: u64,
    pub size: u64,
    pub prefetchable: bool,
}

#[derive(Clone)]
pub struct Bar32 {
    pub address: u32,
    pub size: u32,
    pub prefetchable: bool,
}

#[derive(Debug, Clone)]
pub struct BarIO {
    pub port: u32,
}

pub(crate) trait BarHeader: Sized {
    fn read_bar<C: Chip>(&self, slot: usize, access: &RootComplex<C>) -> Option<Bar>;

    fn address(&self) -> PciAddress;

    fn header_type(&self) -> HeaderType;

    fn parse_bar<C: Chip>(&self, slot_size: usize, access: &RootComplex<C>) -> BarVec {
        let bar0 = match self.read_bar(0, access) {
            Some(bar0) => bar0,
            None => {
                return BarVec::Memory32(BarVecT {
                    data: Vec::new(),
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
        };

        match bar0 {
            Bar::Memory32 {
                address,
                size,
                prefetchable,
            } => {
                let mut v = alloc::vec![None; slot_size];
                v[0] = Some(Bar32 {
                    address,
                    size,
                    prefetchable,
                });

                for i in 1..slot_size {
                    if let Some(Bar::Memory32 {
                        address,
                        size,
                        prefetchable,
                    }) = self.read_bar(i, access)
                    {
                        v[i] = Some(Bar32 {
                            address,
                            size,
                            prefetchable,
                        });
                    }
                }

                BarVec::Memory32(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
            Bar::Memory64 {
                address,
                size,
                prefetchable,
            } => {
                let mut v = alloc::vec![None; slot_size/2];
                v[0] = Some(Bar64 {
                    address,
                    size,
                    prefetchable,
                });

                for i in 1..slot_size / 2 {
                    if let Some(Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    }) = self.read_bar(i * 2, access)
                    {
                        v[i] = Some(Bar64 {
                            address,
                            size,
                            prefetchable,
                        });
                    }
                }
                BarVec::Memory64(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
            Bar::Io { port } => {
                let mut v = alloc::vec![None; slot_size];

                v[0] = Some(BarIO { port });

                for i in 1..slot_size {
                    if let Some(Bar::Io { port }) = self.read_bar(i, access) {
                        v[i] = Some(BarIO { port });
                    }
                }

                BarVec::Io(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
        }
    }
}

impl Debug for Bar32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Bar32 {{ address: {:#p}, size: {:#x}, prefetchable: {}}}",
            self.address as *const u8, self.size, self.prefetchable
        )
    }
}

impl Debug for Bar64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Bar64 {{ address: {:#p}, size: {:#x}, prefetchable: {}}}",
            self.address as *const u8, self.size, self.prefetchable
        )
    }
}

#[derive(Clone)]
pub struct BarVecT<T> {
    data: Vec<Option<T>>,
    address: PciAddress,
    header_type: pci_types::HeaderType,
}

impl<T: Debug> Debug for BarVecT<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BarVecT").field("data", &self.data).finish()
    }
}

impl BarVecT<Bar32> {
    pub(crate) fn set<C: Chip>(
        &self,
        index: usize,
        value: u32,
        access: &RootComplex<C>,
    ) -> core::result::Result<(), BarWriteError> {
        let header = PciHeader::new(self.address);
        match self.header_type {
            pci_types::HeaderType::PciPciBridge => {
                todo!()
            }
            pci_types::HeaderType::Endpoint => unsafe {
                EndpointHeader::from_header(header, access)
                    .unwrap()
                    .write_bar(index as _, access, value as _)
            },
            _ => panic!("Invalid header type"),
        }
    }
}

impl BarVecT<Bar64> {
    pub(crate) fn set<C: Chip>(
        &self,
        index: usize,
        value: u64,
        access: &RootComplex<C>,
    ) -> core::result::Result<(), BarWriteError> {
        let header = PciHeader::new(self.address);
        match self.header_type {
            pci_types::HeaderType::PciPciBridge => {
                todo!()
            }
            pci_types::HeaderType::Endpoint => unsafe {
                debug!("write bar {}: {:#x}", index * 2, value);

                EndpointHeader::from_header(header, access)
                    .unwrap()
                    .write_bar((index * 2) as _, access, value as _)
            },
            _ => panic!("Invalid header type"),
        }
    }
}

impl<T> Index<usize> for BarVecT<T> {
    type Output = Option<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> BarVecT<T> {
    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }
}
