use crate::header::*;

pub trait CardBusBridgeOps: PciHeaderOps {}

pub struct CardBusBridge<C: Chip> {
    pub(crate) header: HeadWrap,
    chip: C,
}

impl<C: Chip> CardBusBridge<C> {
    pub(crate) fn new(header: PciHeader, chip: C) -> Self {
        let header = HeadWrap::new(header);
        Self { header, chip }
    }
}

impl_pci_header!(CardBusBridge);

impl<C: Chip> CardBusBridgeOps for CardBusBridge<C> {}
