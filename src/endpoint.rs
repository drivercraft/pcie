use pci_types::ConfigRegionAccess;

use crate::header::*;
use crate::types::EndpointHeader;

pub trait EndpointOps: PciHeaderOps {
    /// Get the contents of a BAR in a given slot. Empty bars will return `None`.
    ///
    /// ### Note
    /// 64-bit memory BARs use two slots, so if one is decoded in e.g. slot #0, this method should not be called
    /// for slot #1
    fn bar(&self, slot: u8) -> Option<Bar>;

    fn capabilities(&self) -> impl Iterator<Item = PciCapability>;

    fn status(&self) -> StatusRegister;

    fn command(&self) -> CommandRegister;

    fn update_command<F>(&mut self, f: F)
    where
        F: FnOnce(CommandRegister) -> CommandRegister;

    fn capability_pointer(&self) -> u16;

    fn subsystem(&self) -> (SubsystemId, SubsystemVendorId);

    /// Write to a BAR, setting the address for a device to use.
    ///
    /// # Safety
    ///
    /// The supplied value must be a valid BAR value (refer to the PCIe specification for
    /// requirements) and must be of the correct size (i.e. no larger than `u32::MAX` for 32-bit
    /// BARs). In the case of a 64-bit BAR, the supplied slot should be the first slot of the pair.
    unsafe fn write_bar(&mut self, slot: u8, value: usize) -> Result<(), BarWriteError>;

    fn interrupt(&self) -> (InterruptPin, InterruptLine);

    fn update_interrupt<F>(&mut self, f: F)
    where
        F: FnOnce((InterruptPin, InterruptLine)) -> (InterruptPin, InterruptLine);
}

pub struct Endpoint<C: Chip> {
    header: EndpointHeader,
    chip: C,
}

impl<C: Chip> Endpoint<C> {
    pub(crate) fn new(header: PciHeader, chip: C) -> Self {
        let header = EndpointHeader::from_header(header, &chip).unwrap();
        Self { header, chip }
    }
}

impl_pci_header!(Endpoint);

impl<C: Chip> ConfigRegionAccess for Endpoint<C> {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        self.chip.read(address, offset)
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        self.chip.write(address, offset, value)
    }
}

impl<C: Chip> EndpointOps for Endpoint<C> {
    /// Get the contents of a BAR in a given slot. Empty bars will return `None`.
    ///
    /// ### Note
    /// 64-bit memory BARs use two slots, so if one is decoded in e.g. slot #0, this method should not be called
    /// for slot #1
    fn bar(&self, slot: u8) -> Option<Bar> {
        self.header.bar(slot, &self.chip)
    }

    fn capabilities(&self) -> impl Iterator<Item = PciCapability> {
        self.header.capabilities(self.chip.clone())
    }

    fn status(&self) -> StatusRegister {
        self.header.status(&self.chip)
    }

    fn command(&self) -> CommandRegister {
        self.header.command(&self.chip)
    }

    fn update_command<F>(&mut self, f: F)
    where
        F: FnOnce(CommandRegister) -> CommandRegister,
    {
        self.header.update_command(&self.chip, f);
    }

    fn capability_pointer(&self) -> u16 {
        self.header.capability_pointer(&self.chip)
    }

    fn subsystem(&self) -> (SubsystemId, SubsystemVendorId) {
        self.header.subsystem(&self.chip)
    }

    /// Write to a BAR, setting the address for a device to use.
    ///
    /// # Safety
    ///
    /// The supplied value must be a valid BAR value (refer to the PCIe specification for
    /// requirements) and must be of the correct size (i.e. no larger than `u32::MAX` for 32-bit
    /// BARs). In the case of a 64-bit BAR, the supplied slot should be the first slot of the pair.
    unsafe fn write_bar(&mut self, slot: u8, value: usize) -> Result<(), BarWriteError> {
        self.header.write_bar(slot, &self.chip, value)
    }

    fn interrupt(&self) -> (InterruptPin, InterruptLine) {
        self.header.interrupt(&self.chip)
    }

    fn update_interrupt<F>(&mut self, f: F)
    where
        F: FnOnce((InterruptPin, InterruptLine)) -> (InterruptPin, InterruptLine),
    {
        self.header.update_interrupt(&self.chip, f)
    }
}
