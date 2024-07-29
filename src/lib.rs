#![no_std]
mod root;

#[derive(Clone, Copy)]
pub struct PciAddress {
    pub bus: usize,
    pub device: usize,
    pub function: usize,
}
