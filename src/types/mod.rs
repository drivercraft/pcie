use core::fmt::Display;

use bit_field::BitField;
use pci_types::{Bar, CommandRegister, ConfigRegionAccess, EndpointHeader, StatusRegister};

mod bar;

pub use bar::*;
pub use pci_types::{device_type::DeviceType, PciAddress};

macro_rules! struct_header {
    ($name: ident, $($more: tt)*) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub address: PciAddress,
            pub vendor_id: u16,
            pub device_id: u16,
            pub command: CommandRegister,
            pub status: StatusRegister,
            pub has_multiple_functions: bool,
            pub device_revision: u8, pub base_class: u8, pub sub_class:u8, pub interface: u8,
            $($more)*
        }

        impl $name{
            pub fn device_type(&self)->DeviceType{
                DeviceType::from((self.base_class, self.sub_class))
            }
        }
    };
}

#[derive(Debug, Clone)]
pub enum Header {
    PciPciBridge(PciPciBridge),
    Endpoint(Endpoint),
    CardBusBridge(CardBusBridge),
    Unknown(Unknown),
}

impl Display for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Header::PciPciBridge(v) => write!(f, "{}", v),
            Header::Endpoint(v) => write!(f, "{}", v),
            Header::CardBusBridge(_card_bus_bridge) => write!(f, "CardBusBridge"),
            Header::Unknown(unknown) => write!(f, "Unknown({:?})", unknown.kind),
        }
    }
}

struct_header!(Unknown,
    pub kind: u8
);

struct_header!(Endpoint,
    pub bar: BarVec,
);

impl Endpoint {}

impl BarHeader for EndpointHeader {
    fn read_bar<C: crate::Chip>(&self, slot: usize, access: &crate::RootComplex<C>) -> Option<Bar> {
        self.bar(slot as u8, access)
    }

    fn address(&self) -> PciAddress {
        self.header().address()
    }

    fn header_type(&self) -> pci_types::HeaderType {
        pci_types::HeaderType::Endpoint
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Endpoint     {:?} {:#06X}:{:#06X} {:?}",
            self.address,
            self.vendor_id,
            self.device_id,
            self.device_type()
        )?;
        write!(f, "{:?}", self.bar)?;

        Ok(())
    }
}

struct_header!(CardBusBridge,);

struct_header!(PciPciBridge,
    pub primary_bus: u8,
    pub secondary_bus: u8,
    pub subordinate_bus: u8,
);

impl PciPciBridge {
    pub fn update_bus_number<F>(&self, access: impl ConfigRegionAccess, f: F)
    where
        F: FnOnce(BusNumber) -> BusNumber,
    {
        let mut data = unsafe { access.read(self.address, 0x18) };
        let new_bus = f(BusNumber {
            primary: data.get_bits(0..8) as u8,
            secondary: data.get_bits(8..16) as u8,
            subordinate: data.get_bits(16..24) as u8,
        });
        data.set_bits(16..24, new_bus.subordinate.into());
        data.set_bits(8..16, new_bus.secondary.into());
        data.set_bits(0..8, new_bus.primary.into());
        unsafe {
            access.write(self.address, 0x18, data);
        }
    }

    pub fn sync_bus_number(&self, access: impl ConfigRegionAccess) {
        if self.secondary_bus > 0 {
            self.update_bus_number(access, |mut bus| {
                bus.primary = self.primary_bus;
                bus.secondary = self.secondary_bus;
                bus.subordinate = self.subordinate_bus;
                bus
            });
        }
    }
}

impl Display for PciPciBridge {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "PciPciBridge {:?} {:#06X}:{:#06X} {:?}",
            self.address,
            self.vendor_id,
            self.device_id,
            self.device_type()
        )?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BusNumber {
    pub primary: u8,
    pub secondary: u8,
    pub subordinate: u8,
}
