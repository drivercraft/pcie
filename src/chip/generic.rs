use super::{Chip, ConfigRegionAccess};
use crate::PciAddress;

#[derive(Clone)]
pub struct Generic {
    mmio_base: usize,
}

impl Generic {}

impl ConfigRegionAccess for Generic {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let ptr = self.mmio_addr(address, offset);
        ptr.as_ptr().read_volatile()
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let ptr = self.mmio_addr(address, offset);
        ptr.as_ptr().write_volatile(value);
    }
}

impl Chip for Generic {
    fn new(mmio_base: usize) -> Self {
        Self { mmio_base }
    }
    
    fn mmio_base(&self)->usize {
        self.mmio_base
    }
}
