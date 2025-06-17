#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate bare_test;

#[bare_test::tests]
mod tests {
    use bare_test::{
        fdt_parser::PciSpace,
        globals::{global_val, PlatformInfoKind},
        mem::iomap,
        println,
    };
    use log::info;
    use pcie::{RootComplexGeneric, SimpleBarAllocator};

    #[test]
    fn test_iter() {
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt = fdt.get();

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
            info!("{range:?}");
            match range.space {
                PciSpace::Memory32 => bar_alloc.set_mem32(range.cpu_address as _, range.size as _),
                PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
                _ => {}
            }
        }

        let base_vaddr = pcie_regs[0];

        info!("Init PCIE @{base_vaddr:?}");

        let mut root = RootComplexGeneric::new(base_vaddr);

        for header in root.enumerate(None, Some(bar_alloc)) {
            println!("{}", header);
        }

        for header in root.enumerate_keep_bar(None) {
            if let pcie::Header::Endpoint(endpoint) = header.header {
                endpoint.update_command(header.root, |cmd| cmd);
            }
        }

        println!("test passed!");
    }
}
