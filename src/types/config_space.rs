use core::fmt::Display;

use enum_dispatch::enum_dispatch;
use pci_types::PciHeader;

use crate::PcieController;









pub struct PciConfigSpace {
    vid: u16,
    did: u16,
    root: PcieController,
    header: PciHeader,
}

impl PciConfigSpace {
    pub(crate) fn new(root: PcieController, header: PciHeader) -> Self {
        let access = &root;
        let (vid, did) = header.id(access);
        Self {
            vid,
            did,
            root,
            header,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        self.vid
    }
    pub fn device_id(&self) -> u16 {
        self.did
    }
}

impl Display for PciConfigSpace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PciConfigSpace")
            .field("vid", &format_args!("{:#06x}", self.vid))
            .field("did", &format_args!("{:#06x}", self.did))
            .finish()
    }
}
