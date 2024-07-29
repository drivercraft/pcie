use crate::PciAddress;
use core::{marker::PhantomData, ops::Range, ptr::NonNull};

pub trait Chip {
    fn map_conf(mmio_base: NonNull<u8>, addr: PciAddress) -> Option<usize>;
}

/// The root complex of a PCI bus.
#[derive(Clone)]
pub struct PciRootComplex<C: Chip> {
    mmio_base: NonNull<u8>,
    bar_range: Range<u64>,
    _marker: PhantomData<C>,
}

impl<C: Chip> PciRootComplex<C> {
    pub fn new(mmio_base: NonNull<u8>, bar_range: Range<u64>) -> Self {
        Self {
            mmio_base,
            _marker: PhantomData::default(),
            bar_range,
        }
    }
}
