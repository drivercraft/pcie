use crate::header::*;

pub struct Unknown<C: Chip> {
    kind_id: u8,
    header: HeadWrap,
    chip: C,
}

impl<C: Chip> Unknown<C> {
    pub(crate) fn new(header: PciHeader, chip: C, id: u8) -> Self {
        let header = HeadWrap::new(header);
        Self {
            kind_id: id,
            header,
            chip,
        }
    }
}

impl_pci_header!(Unknown);

pub trait UnknownOps: PciHeaderOps {
    fn kind(&self) -> u8;
}

impl<C: Chip> UnknownOps for Unknown<C> {
    fn kind(&self) -> u8 {
        self.kind_id
    }
}
