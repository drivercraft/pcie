use alloc::vec::Vec;

use crate::{Chip, Header, PciAddress};
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

    pub fn enumerate(&mut self, range: Option<Range<usize>>) -> PciIterator<'_, C> {
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

    pub fn read_config(&self, address: PciAddress, offset: u16) -> u32 {
        unsafe { self.chip.read(self.mmio_base, address, offset) }
    }

    pub fn write_config(&mut self, address: PciAddress, offset: u16, value: u32) {
        unsafe { self.chip.write(self.mmio_base, address, offset, value) }
    }
}

pub struct PciIterator<'a, C: Chip> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: &'a mut RootComplex<C>,
    segment: u16,
    bus: u8,
    bus_max: u8,
    device: u8,
    function: u8,
    bus_iter: u8,
}

impl<'a, C: Chip> Iterator for PciIterator<'a, C> {
    type Item = Header;

    fn next(&mut self) -> Option<Self::Item> {
        // loop {
        //     let curent_addr = self.current_addr();
        //     let header = types::Header::new(self.root, curent_addr);
        // }

        None
    }
}

impl<'a, C: Chip> PciIterator<'a, C> {
    fn current_addr(&self) -> PciAddress {
        PciAddress::new(self.segment, self.bus, self.device, self.function)
    }
}
