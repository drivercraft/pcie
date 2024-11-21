use core::ptr::NonNull;

use crate::{err::*, PciAddress};

pub mod generic;

pub trait Chip: Send {
    fn init(&mut self) -> Result;

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
    unsafe fn write(
        &mut self,
        mmio_base: NonNull<u8>,
        address: PciAddress,
        offset: u16,
        value: u32,
    );
}
