use core::fmt::Display;

pub use crate::types::{
    capability::CapabilityIterator, device_type::DeviceType, Bar, BarWriteError, CommandRegister,
    InterruptLine, InterruptPin, PciAddress, StatusRegister, SubsystemId, SubsystemVendorId,
};
use crate::types::{EndpointHeader, PciHeader, PciPciBridgeHeader};
use crate::Chip;
use derive_getters::Getters;

pub struct PciDevice<C: Chip> {
    chip: C,
    header: PciHeader,
    /// The PCI vendor ID.
    vendor_id: u16,
    /// The PCI device ID.
    device_id: u16,
    /// The PCI class.
    class: u8,
    /// The PCI subclass.
    subclass: u8,
    /// The PCI programming interface byte.
    interface: u8,
    /// The PCI revision ID.
    revision: u8,
    /// The PCI device kind.
    kind: PciDeviceKind<C>,
}

impl<C: Chip> PciDevice<C> {
    pub(crate) fn new(chip: C, header_ref: &PciHeader) -> Self {
        let header = PciHeader::new(header_ref.address());
        let (vendor_id, device_id) = header.id(&chip);
        let (revision, class, subclass, interface) = header.revision_and_class(&chip);
        let header_type = header.header_type(&chip);
        let kind = match header_type {
            pci_types::HeaderType::Endpoint => {
                PciDeviceKind::Endpoint(Endpoint::new(header, &chip))
            }
            pci_types::HeaderType::PciPciBridge => {
                PciDeviceKind::PciPciBridge(PciPciBridge::new(header, &chip))
            }
            pci_types::HeaderType::CardBusBridge => todo!(),
            pci_types::HeaderType::Unknown(_) => todo!(),
            _ => todo!(),
        };
        let header = PciHeader::new(header_ref.address());
        Self {
            chip,
            header,
            vendor_id,
            device_id,
            class,
            subclass,
            interface,
            revision,
            kind,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub fn device_id(&self) -> u16 {
        self.device_id
    }

    pub fn class(&self) -> u8 {
        self.class
    }

    pub fn subclass(&self) -> u8 {
        self.subclass
    }

    pub fn address(&self) -> PciAddress {
        self.header.address()
    }

    pub fn interface(&self) -> u8 {
        self.interface
    }

    pub fn revision(&self) -> u8 {
        self.revision
    }

    pub fn kind<'a>(&'a self) -> &'a PciDeviceKind<C> {
        &self.kind
    }

    pub fn device_type(&self) -> DeviceType {
        DeviceType::from((self.class, self.subclass))
    }
}

impl<C: Chip> Display for PciDevice<C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} {:#X}:{:#X} {:?} ",
            self.header.address(),
            self.vendor_id,
            self.device_id,
            self.device_type()
        )
    }
}

pub enum PciDeviceKind<C: Chip> {
    PciPciBridge(PciPciBridge<C>),
    Endpoint(Endpoint<C>),
}

pub struct PciPciBridge<C: Chip> {
    pub(crate) header: PciPciBridgeHeader,
    chip: C,
}
impl<C: Chip> PciPciBridge<C> {
    fn new(header: PciHeader, chip: &C) -> Self {
        let header = PciPciBridgeHeader::from_header(header, chip).unwrap();
        Self {
            header,
            chip: chip.clone(),
        }
    }
}

pub struct Endpoint<C: Chip> {
    header: EndpointHeader,
    chip: C,
}

impl<C: Chip> Endpoint<C> {
    fn new(header: PciHeader, chip: &C) -> Self {
        let header = EndpointHeader::from_header(header, chip).unwrap();
        Self {
            header,
            chip: chip.clone(),
        }
    }

    /// Get the contents of a BAR in a given slot. Empty bars will return `None`.
    ///
    /// ### Note
    /// 64-bit memory BARs use two slots, so if one is decoded in e.g. slot #0, this method should not be called
    /// for slot #1
    pub fn bar(&self, slot: u8) -> Option<Bar> {
        self.header.bar(slot, &self.chip)
    }

    pub fn capabilities(&self) -> CapabilityIterator<C> {
        self.header.capabilities(self.chip.clone())
    }

    pub fn status(&self) -> StatusRegister {
        self.header.status(&self.chip)
    }

    pub fn command(&self) -> CommandRegister {
        self.header.command(&self.chip)
    }

    pub fn update_command<F>(&mut self, f: F)
    where
        F: FnOnce(CommandRegister) -> CommandRegister,
    {
        self.header.update_command(&self.chip, f);
    }

    pub fn capability_pointer(&self) -> u16 {
        self.header.capability_pointer(&self.chip)
    }

    pub fn subsystem(&self) -> (SubsystemId, SubsystemVendorId) {
        self.header.subsystem(&self.chip)
    }

    /// Write to a BAR, setting the address for a device to use.
    ///
    /// # Safety
    ///
    /// The supplied value must be a valid BAR value (refer to the PCIe specification for
    /// requirements) and must be of the correct size (i.e. no larger than `u32::MAX` for 32-bit
    /// BARs). In the case of a 64-bit BAR, the supplied slot should be the first slot of the pair.
    pub unsafe fn write_bar(&mut self, slot: u8, value: usize) -> Result<(), BarWriteError> {
        self.header.write_bar(slot, &self.chip, value)
    }

    pub fn interrupt(&self) -> (InterruptPin, InterruptLine) {
        self.header.interrupt(&self.chip)
    }

    pub fn update_interrupt<F>(&mut self, f: F)
    where
        F: FnOnce((InterruptPin, InterruptLine)) -> (InterruptPin, InterruptLine),
    {
        self.header.update_interrupt(&self.chip, f)
    }
}
