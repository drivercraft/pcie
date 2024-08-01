use crate::header::*;
use crate::types::PciPciBridgeHeader;

pub trait PciPciBridgeOps: PciHeaderOps {
    fn primary_bus_number(&self) -> u8;
    fn secondary_bus_number(&self) -> u8;
    fn subordinate_bus_number(&self) -> u8;
}

pub struct PciPciBridge<C: Chip> {
    pub(crate) header: PciPciBridgeHeader,
    chip: C,
}

impl<C: Chip> PciPciBridge<C> {
    pub(crate) fn new(header: PciHeader, chip: C) -> Self {
        let header = PciPciBridgeHeader::from_header(header, &chip).unwrap();
        Self { header, chip }
    }
}

impl_pci_header!(PciPciBridge);

impl<C: Chip> PciPciBridgeOps for PciPciBridge<C> {
    fn primary_bus_number(&self) -> u8 {
        self.header.primary_bus_number(&self.chip)
    }

    fn secondary_bus_number(&self) -> u8 {
        self.header.secondary_bus_number(&self.chip)
    }

    fn subordinate_bus_number(&self) -> u8 {
        self.header.subordinate_bus_number(&self.chip)
    }
}
