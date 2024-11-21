use crate::Chip;
use core::ptr::NonNull;

pub struct RootComplex<C: Chip> {
    chip: C,
    mmio_base: NonNull<u8>,
}

impl<C> RootComplex<C>
where
    C: Chip,
{
    pub fn new_with_chip(mmio_base: NonNull<u8>, chip: C) -> Self {
        Self { chip, mmio_base }
    }
}
