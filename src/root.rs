use alloc::vec::Vec;

use crate::{endpoiont::PciEndpoint, Chip};
use core::{ops::Range, ptr::NonNull};

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

    pub fn enumerate(&self, range: Option<Range<usize>>) -> PciIterator<'_, C> {
        let range = range.unwrap_or_else(|| 0..0x100);

        PciIterator {
            root: self,
            segment: 0,
            bus: range.start as _,
            bus_max: (range.end - 1) as _,
            device: 0,
            function: 0,
            bus_iter: 0,
        }
    }
}

pub struct PciIterator<'a, C: Chip> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: &'a RootComplex<C>,
    segment: u16,
    bus: u8,
    bus_max: u8,
    device: u8,
    function: u8,
    bus_iter: u8,
}

impl<'a, C: Chip> Iterator for PciIterator<'a, C> {
    type Item = PciDevice<'a, C>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub enum PciDevice<'a, C: Chip> {
    Endpoint(PciEndpoint<'a, C>),
}
