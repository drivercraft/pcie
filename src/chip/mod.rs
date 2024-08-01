use crate::types::{ConfigRegionAccess, PciAddress};
use core::ptr::NonNull;
pub use embedded_hal::delay::DelayNs;

pub mod generic;
pub trait Chip: ConfigRegionAccess + Send + Clone + 'static {
    fn new(mmio_base: usize) -> Self;
    fn mmio_base(&self) -> usize;

    fn mmio_addr(&self, address: PciAddress, offset: u16) -> NonNull<u32> {
        let address = (address.bus() as u32) << 20
            | (address.device() as u32) << 15
            | (address.function() as u32) << 12
            | offset as u32;
        unsafe {
            let ptr = (self.mmio_base() as *mut u32).add((address >> 2) as usize);
            NonNull::new_unchecked(ptr as *mut _)
        }
    }
}
