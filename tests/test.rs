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
    use pcie::{PciSpace32, PciSpace64, RootComplex, RootComplexGeneric, SimpleBarAllocator};

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

        for reg in pcie.node.reg().unwrap() {
            println!("pcie reg: {:#x}", reg.address);
            pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
        }

        let mut space32 = None;
        let mut space64 = None;

        for range in pcie.ranges().unwrap() {
            info!("{range:?}");
            match range.space {
                PciSpace::Memory32 => {
                    space32 = Some(PciSpace32 {
                        address: range.cpu_address as u32,
                        size: range.size as _,
                        prefetchable: range.prefetchable,
                    })
                }
                PciSpace::Memory64 => {
                    space64 = Some(PciSpace64 {
                        address: range.cpu_address,
                        size: range.size,
                        prefetchable: range.prefetchable,
                    })
                }
                _ => {}
            }
        }

        let base_vaddr = pcie_regs[0];

        info!("Init PCIE @{base_vaddr:?}");

        let mut root = RootComplex::new_generic(base_vaddr, space32, space64);
        // 通过 RootComplex 的 allocator 字段进行资源配置

        for header in root.enumerate(None) {
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
