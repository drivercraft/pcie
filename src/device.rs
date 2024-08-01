use core::fmt::Display;
use core::ops::Deref;

use crate::{
    endpoint::Endpoint, header::PciHeaderOps, pcipcibridge::PciPciBridge, types::PciHeader, Chip,
};

pub enum PciDevice<C: Chip> {
    Endpoint(Endpoint<C>),
    PciPciBridge(PciPciBridge<C>),
    Unknown,
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
            pci_types::HeaderType::CardBusBridge => todo!(),
            pci_types::HeaderType::Unknown(_) => todo!(),
            _ => todo!(),
        }
    }

    fn as_ref(&self) -> &dyn PciHeaderOps {
        match self {
            PciDevice::Endpoint(ep) => ep,
            PciDevice::PciPciBridge(br) => br,
            PciDevice::Unknown => todo!(),
        }
    }
}

impl<C: Chip> Deref for PciDevice<C> {
    type Target = dyn PciHeaderOps;

    fn deref(&self) -> &Self::Target {
        match self {
            PciDevice::Endpoint(ep) => ep,
            PciDevice::PciPciBridge(br) => br,
            PciDevice::Unknown => todo!(),
        }
    }
}

impl<C: Chip> Display for PciDevice<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (v, d) = self.as_ref().id();
        write!(
            f,
            "{:?} {:04X}:{:04X} {:?} ",
            self.as_ref().address(),
            v,
            d,
            self.as_ref().device_type()
        )
    }
}
