#![no_std]

extern crate alloc;

extern crate log;

mod addr_alloc;
mod bar_alloc;
mod chip;
pub mod err;
mod root;
mod types;

pub use chip::{
    generic::{Generic, RootComplexGeneric},
    PcieController, PcieGeneric,
};

pub use bar_alloc::*;
pub use root::{EnumElem, RootComplex};
pub use types::*;

#[derive(Clone, Copy, Debug)]
pub struct PciSpace32 {
    pub address: u32,
    pub size: u32,
    pub prefetchable: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PciSpace64 {
    pub address: u64,
    pub size: u64,
    pub prefetchable: bool,
}
