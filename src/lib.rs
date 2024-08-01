#![no_std]

extern crate alloc;
mod chip;
mod device;
mod root;
#[macro_use]
mod header;
pub(crate) mod cardbusbridge;
pub(crate) mod endpoint;
pub(crate) mod pcipcibridge;
pub mod preludes;
pub(crate) mod types;
pub(crate) mod unknown;

pub use chip::*;
pub use device::*;
pub use root::RootComplex;
use types::PciAddress;

pub use cardbusbridge::CardBusBridge;
pub use endpoint::Endpoint;
pub use pcipcibridge::PciPciBridge;
pub use unknown::Unknown;

pub type RootGeneric = RootComplex<generic::Generic>;
