#![no_std]

extern crate alloc;

mod bar_alloc;
mod chip;
pub mod err;
mod root;
mod types;

pub use chip::{
    generic::{Generic, RootComplexGeneric},
    Chip,
};

pub use bar_alloc::*;
pub use root::RootComplex;
pub use types::*;

pub trait BarAllocator {
    fn alloc_memory32(&mut self, size: u32) -> Option<u32>;
    fn alloc_memory64(&mut self, size: u64) -> Option<u64>;
}
