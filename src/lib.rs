#![no_std]

extern crate alloc;
mod chip;
mod device;
mod root;
pub(crate) mod types;

pub use chip::*;
pub use device::*;
use root::RootComplex;

pub type RootGeneric = RootComplex<generic::Generic>;
