use bit_field::BitField;
pub use pci_types::*;

pub(crate) trait PciPciBridgeHeaderWrite {
    fn address(&self) -> PciAddress;

    fn set_primary_bus_number(&self, val: u8, access: impl ConfigRegionAccess) {
        let address = self.address();
        let mut data = unsafe { access.read(address, 0x18) };
        data.set_bits(0..8, val as _);
        unsafe { access.write(address, 0x18, data) }
    }

    fn set_secondary_bus_number(&self, val: u8, access: impl ConfigRegionAccess) {
        let address = self.address();
        let mut data = unsafe { access.read(address, 0x18) };
        data.set_bits(8..16, val as _);
        unsafe { access.write(address, 0x18, data) }
    }

    fn set_subordinate_bus_number(&self, val: u8, access: impl ConfigRegionAccess) {
        let address = self.address();
        let mut data = unsafe { access.read(address, 0x18) };
        data.set_bits(16..24, val as _);
        unsafe { access.write(address, 0x18, data) }
    }
}

impl PciPciBridgeHeaderWrite for pci_types::PciPciBridgeHeader {
    fn address(&self) -> PciAddress {
        self.header().address()
    }
}
