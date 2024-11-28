#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_test::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bare_test::{driver::device_tree::get_device_tree, fdt::PciSpace, mem::mmu::iomap, println};
use log::info;
use pcie::{RootComplexGeneric, SimpleBarAllocator};

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

    let mut bar_alloc = SimpleBarAllocator::default();

    for reg in pcie.node.reg().unwrap() {
        println!("pcie reg: {:#x}", reg.address);
        pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
    }

    for range in pcie.ranges().unwrap() {
        info!("{:?}", range);
        match range.space {
            PciSpace::Memory32 => bar_alloc.set_mem32(range.cpu_address as _, range.size as _),
            PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
            _ => {}
        }
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let mut root = RootComplexGeneric::new(base_vaddr);

    for header in root.enumerate(None, Some(bar_alloc)) {
        println!("{}", header);
    }

    for header in root.enumerate_keep_bar(None) {
        match header.header {
            pcie::Header::Endpoint(endpoint) => {
                endpoint.update_command(header.root, |cmd| cmd);
            }
            _ => {}
        }
    }

    println!("test passed!");
}
