pub(crate) use crate::types::PciHeader;
pub use crate::{
    types::{
        capability::PciCapability, device_type::DeviceType, Bar, BarWriteError, BaseClass,
        CommandRegister, DeviceRevision, Interface, InterruptLine, InterruptPin, PciAddress,
        StatusRegister, SubClass, SubsystemId, SubsystemVendorId,
    },
    Chip,
};

pub trait PciHeaderOps: Send {
    fn id(&self) -> (u16, u16);

    fn address(&self) -> PciAddress;

    fn revision_and_class(&self) -> (DeviceRevision, BaseClass, SubClass, Interface);

    fn device_type(&self) -> DeviceType {
        let (_, class, subclass, _) = self.revision_and_class();
        DeviceType::from((class, subclass))
    }
}
pub(crate) struct HeadWrap(PciHeader);
impl HeadWrap {
    pub fn new(header: PciHeader) -> Self {
        Self(header)
    }

    pub fn header(&self) -> &PciHeader {
        &self.0
    }
}
macro_rules! impl_pci_header {
    ($t:ident) => {
        impl<C> PciHeaderOps for $t<C>
        where
            C: Chip,
        {
            fn id(&self) -> (u16, u16) {
                self.header.header().id(&self.chip)
            }

            fn revision_and_class(
                &self,
            ) -> (
                crate::types::DeviceRevision,
                crate::types::BaseClass,
                crate::types::SubClass,
                crate::types::Interface,
            ) {
                self.header.header().revision_and_class(&self.chip)
            }

            fn address(&self) -> crate::PciAddress {
                self.header.header().address()
            }
        }
    };
}
