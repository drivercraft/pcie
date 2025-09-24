use core::{cell::UnsafeCell, ptr::NonNull};

use alloc::{boxed::Box, sync::Arc};
use pci_types::ConfigRegionAccess;

use crate::PciAddress;

pub mod generic;

pub trait Chip: Send {
    /// Performs a PCI read at `address` with `offset`.
    ///
    /// # Safety
    ///
    /// `address` and `offset` must be valid for PCI reads.
    unsafe fn read(&self, mmio_base: NonNull<u8>, address: PciAddress, offset: u16) -> u32;

    /// Performs a PCI write at `address` with `offset`.
    ///
    /// # Safety
    ///
    /// `address` and `offset` must be valid for PCI writes.
    unsafe fn write(&self, mmio_base: NonNull<u8>, address: PciAddress, offset: u16, value: u32);
}

pub trait Controller: Send + 'static {
    /// Performs a PCI read at `address` with `offset`.
    ///
    /// # Safety
    ///
    /// `address` and `offset` must be valid for PCI reads.
    fn read(&mut self, address: PciAddress, offset: u16) -> u32;

    /// Performs a PCI write at `address` with `offset`.
    ///
    /// # Safety
    ///
    /// `address` and `offset` must be valid for PCI writes.
    fn write(&mut self, address: PciAddress, offset: u16, value: u32);
}

#[derive(Clone)]
pub struct PcieController {
    chip: Arc<ChipRaw>,
}

impl PcieController {
    pub fn new(chip: impl Controller) -> Self {
        Self {
            chip: Arc::new(ChipRaw::new(chip)),
        }
    }
}

impl ConfigRegionAccess for PcieController {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        unsafe { (*self.chip.0.get()).read(address, offset) }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        unsafe { (*self.chip.0.get()).write(address, offset, value) }
    }
}

struct ChipRaw(UnsafeCell<Box<dyn Controller>>);

unsafe impl Send for ChipRaw {}
unsafe impl Sync for ChipRaw {}

impl ChipRaw {
    fn new(chip: impl Controller) -> Self {
        Self(UnsafeCell::new(Box::new(chip)))
    }
}

pub struct PcieGeric {
    mmio_base: NonNull<u8>,
}

unsafe impl Send for PcieGeric {}

impl PcieGeric {
    pub fn new(mmio_base: NonNull<u8>) -> Self {
        Self { mmio_base }
    }

    fn mmio_addr(&self, mmio_base: NonNull<u8>, address: PciAddress, offset: u16) -> NonNull<u32> {
        let address = (address.bus() as u32) << 20
            | (address.device() as u32) << 15
            | (address.function() as u32) << 12
            | offset as u32;
        unsafe {
            let ptr: NonNull<u32> = mmio_base.cast().add((address >> 2) as usize);
            ptr
        }
    }
}

impl Controller for PcieGeric {
    fn read(&mut self, address: PciAddress, offset: u16) -> u32 {
        let ptr = self.mmio_addr(self.mmio_base, address, offset);
        unsafe { ptr.as_ptr().read_volatile() }
    }

    fn write(&mut self, address: PciAddress, offset: u16, value: u32) {
        let ptr = self.mmio_addr(self.mmio_base, address, offset);
        unsafe { ptr.as_ptr().write_volatile(value) }
    }
}
