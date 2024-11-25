use bit_field::BitField;
use pci_types::{CommandRegister, ConfigRegionAccess, StatusRegister};

pub use pci_types::PciAddress;

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
            $($more)*
        }
    };
}

#[derive(Debug, Clone)]
pub enum Header {
    PciPciBridge(PciPciBridge),
    Endpoint(Endpoint),
    Unknown(Unknown),
}

struct_header!(Unknown,
    pub kind: u8
);

struct_header!(Endpoint,);

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
        self.update_bus_number(access, |mut bus| {
            bus.primary = self.primary_bus;
            bus.secondary = self.secondary_bus;
            bus.subordinate = self.subordinate_bus;
            bus
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BusNumber {
    pub primary: u8,
    pub secondary: u8,
    pub subordinate: u8,
}
