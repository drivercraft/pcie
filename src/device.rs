use core::fmt::Display;
use core::ops::Deref;

use crate::{
    cardbusbridge::CardBusBridge, endpoint::Endpoint, header::PciHeaderOps,
    pcipcibridge::PciPciBridge, types::PciHeader, unknown::Unknown, Chip,
};

pub enum PciDevice<C: Chip> {
    Endpoint(Endpoint<C>),
    PciPciBridge(PciPciBridge<C>),
    CardBusBridge(CardBusBridge<C>),
    Unknown(Unknown<C>),
}
impl<C: Chip> PciDevice<C> {
    pub(crate) fn new(chip: C, header_ref: &PciHeader) -> Self {
        let header = PciHeader::new(header_ref.address());

        let header_type = header.header_type(&chip);
        match header_type {
            pci_types::HeaderType::Endpoint => PciDevice::Endpoint(Endpoint::new(header, chip)),
            pci_types::HeaderType::PciPciBridge => {
                PciDevice::PciPciBridge(PciPciBridge::new(header, chip))
            }
            pci_types::HeaderType::CardBusBridge => {
                PciDevice::CardBusBridge(CardBusBridge::new(header, chip))
            }
            pci_types::HeaderType::Unknown(id) => {
                PciDevice::Unknown(Unknown::new(header, chip, id))
            }
            _ => todo!(),
        }
    }
}

impl<C: Chip> Deref for PciDevice<C> {
    type Target = dyn PciHeaderOps;

    fn deref(&self) -> &Self::Target {
        match self {
            PciDevice::Endpoint(ep) => ep,
            PciDevice::PciPciBridge(br) => br,
            PciDevice::Unknown(u) => u,
            PciDevice::CardBusBridge(card) => card,
        }
    }
}

impl<C: Chip> Display for PciDevice<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (v, d) = self.id();
        write!(
            f,
            "{:?} {:04X}:{:04X} {:?} ",
            self.address(),
            v,
            d,
            self.device_type()
        )
    }
}
