use core::ptr::NonNull;

use crate::{root::RootComplex, PciAddress};

use super::Chip;

pub struct Generic {}

impl Generic {}

impl Chip for Generic {
    unsafe fn read(&self, mmio_base: NonNull<u8>, address: PciAddress, offset: u16) -> u32 {
        let ptr = self.mmio_addr(mmio_base, address, offset);
        ptr.as_ptr().read_volatile()
    }

    unsafe fn write(
        &self,
        mmio_base: NonNull<u8>,
        address: PciAddress,
        offset: u16,
        value: u32,
    ) {
        let ptr = self.mmio_addr(mmio_base, address, offset);
        ptr.as_ptr().write_volatile(value);
    }

    fn init(&mut self) -> super::Result {
        Ok(())
    }
}

impl Generic {
    fn mmio_addr(&self, mmio_base: NonNull<u8>, address: PciAddress, offset: u16) -> NonNull<u32> {
        let address = (address.bus() as u32) << 20
            | (address.device() as u32) << 15
            | (address.function() as u32) << 12
            | offset as u32;
        unsafe {
            let ptr = mmio_base.add((address >> 2) as usize);
            ptr.cast()
        }
    }
}

pub type RootComplexGeneric = RootComplex<Generic>;

impl RootComplexGeneric {
    pub fn new(mmio_base: NonNull<u8>) -> Self {
        RootComplex::new_with_chip(mmio_base, Generic {})
    }
}
