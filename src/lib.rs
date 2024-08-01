#![no_std]

extern crate alloc;
mod chip;
mod device;
mod root;
#[macro_use]
mod header;
pub(crate) mod endpoint;
pub(crate) mod pcipcibridge;
pub mod preludes;
pub(crate) mod types;

pub use chip::*;
pub use device::*;
pub use root::RootComplex;
use types::PciAddress;

pub type RootGeneric = RootComplex<generic::Generic>;
