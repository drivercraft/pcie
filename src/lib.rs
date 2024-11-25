#![no_std]

extern crate alloc;

mod chip;
pub mod err;
mod root;
mod types;

pub use chip::{
    generic::{Generic, RootComplexGeneric},
    Chip,
};

pub use root::RootComplex;
pub use types::*;
