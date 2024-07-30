#![no_std]

extern crate alloc;
mod chip;
mod root;
mod types;

pub use chip::*;
use root::RootComplex;
pub use types::*;


pub type RootGeneric = RootComplex<generic::Generic>;
