#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bare_test::{driver::device_tree::get_device_tree, fdt::PciSpace, mem::mmu::iomap, println};
use log::info;
use pcie::RootComplexGeneric;

extern crate alloc;
extern crate bare_test;

#[bare_test::entry]
fn main() {
    test_main();
}

#[test_case]
fn test_pcie() {
    let fdt = get_device_tree().unwrap();
    let pcie = fdt
        .find_compatible(&["pci-host-ecam-generic"])
        .next()
        .unwrap()
        .into_pci()
        .unwrap();

    let mut pcie_regs = alloc::vec![];

    println!("test nvme");

    println!("pcie: {}", pcie.node.name);

    for reg in pcie.node.reg().unwrap() {
        println!("pcie reg: {:#x}", reg.address);
        pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
    }

    let mut m32_range = 0..0;
    let mut m64_range = 0..0;

    for range in pcie.ranges().unwrap() {
        match range.space {
            PciSpace::Memory32 => m32_range = range.cpu_address..range.size,
            PciSpace::Memory64 => m64_range = range.cpu_address..range.size,
            _ => {}
        }
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let mut root = RootComplexGeneric::new(base_vaddr);

    for header in root.enumerate(None) {
        println!("header: {:?}", header);
    }

    println!("test passed!");
}
