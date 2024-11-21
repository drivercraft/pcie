#![no_std]

extern crate alloc;

mod address;
mod chip;
mod root;
pub mod err;

pub use address::PciAddress;
pub use chip::{
    generic::{Generic, RootComplexGeneric},
    Chip,
};
