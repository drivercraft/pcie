#![no_std]

extern crate alloc;

mod chip;
pub mod err;
mod root;
mod types;
mod bar_alloc;

pub use chip::{
    generic::{Generic, RootComplexGeneric},
    Chip,
};

pub use root::RootComplex;
pub use types::*;
pub use bar_alloc::*;

pub trait BarAllocator {
    fn alloc_memory32(&mut self, size: u32) -> Option<u32>;
    fn alloc_memory64(&mut self, size: u64) -> Option<u64>;
}
