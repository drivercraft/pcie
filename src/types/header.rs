use core::fmt::Debug;

use alloc::format;
use bit_field::BitField;
pub use bridge::PciBridge;
pub use endpoint::Endpoint;

mod bridge;
mod endpoint;

use crate::{err::*, Chip, PciAddress, RootComplex};

/// Every PCI configuration region starts with a header made up of two parts:
///    - a predefined region that identify the function (bytes `0x00..0x10`)
///    - a device-dependent region that depends on the Header Type field
///
/// The predefined region is of the form:
/// ```ignore
///     32                            16                              0
///      +-----------------------------+------------------------------+
///      |       Device ID             |       Vendor ID              | 0x00
///      |                             |                              |
///      +-----------------------------+------------------------------+
///      |         Status              |       Command                | 0x04
///      |                             |                              |
///      +-----------------------------+---------------+--------------+
///      |               Class Code                    |   Revision   | 0x08
///      |                                             |      ID      |
///      +--------------+--------------+---------------+--------------+
///      |     BIST     |    Header    |    Latency    |  Cacheline   | 0x0c
///      |              |     type     |     timer     |    size      |
///      +--------------+--------------+---------------+--------------+
/// ```
#[derive(Clone)]
pub struct Header {
    address: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: Command,
    pub status: PciStatus,
    pub revision: u8,
    pub class: u8,
    pub subclass: u8,
    pub interface: u8,
}

impl Header {
    pub fn new<C: Chip>(root: &RootComplex<C>, address: PciAddress) -> Self {
        let id = root.read_config(address, 0);
        let vendor_id = id.get_bits(0..16) as u16;
        let device_id = id.get_bits(16..32) as u16;

        let command =
            Command::from_bits_retain(root.read_config(address, 0x4).get_bits(0..16) as u16);

        let status = PciStatus::new(root.read_config(address, 0x4).get_bits(16..32) as u16);

        let r: RevisionAndClass = unsafe { core::mem::transmute(root.read_config(address, 0x08)) };

        Self {
            address,
            vendor_id,
            device_id,
            command,
            status,
            revision: r.revision,
            class: r.class,
            subclass: r.subclass,
            interface: r.interface,
        }
    }

    pub fn header_type<C: Chip>(&self, root: &RootComplex<C>) -> HeaderType {
        /*
         * Read bits 0..=6 of the Header Type. Bit 7 dictates whether the device has multiple functions and so
         * isn't returned here.
         */
        match root.read_config(self.address, 0x0c).get_bits(16..23) {
            0x00 => HeaderType::Endpoint(Endpoint {}),
            0x01 => HeaderType::PciBridge(PciBridge {}),
            t => HeaderType::Unknown(t as u8),
        }
    }
}

impl Debug for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Header")
            .field("address", &self.address)
            .field("vendor_id", &self.vendor_id)
            .field("device_id", &self.device_id)
            .field("command", &self.command)
            .field("status", &self.status)
            .field("revision", &self.revision)
            .field("class", &self.class)
            .field("subclass", &self.subclass)
            .field("interface", &self.interface)
            .finish()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RevisionAndClass {
    pub revision: u8,
    pub class: u8,
    pub subclass: u8,
    pub interface: u8,
}

#[derive(Clone, Copy)]
pub enum HeaderType {
    Endpoint(Endpoint),
    PciBridge(PciBridge),
    Unknown(u8),
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PciStatus(u16);

impl PciStatus {
    pub fn new(value: u16) -> Self {
        PciStatus(value)
    }

    /// Will be `true` whenever the device detects a parity error, even if parity error handling is disabled.
    pub fn parity_error_detected(&self) -> bool {
        self.0.get_bit(15)
    }

    /// Will be `true` whenever the device asserts SERR#.
    pub fn signalled_system_error(&self) -> bool {
        self.0.get_bit(14)
    }

    /// Will return `true`, by a master device, whenever its transaction
    /// (except for Special Cycle transactions) is terminated with Master-Abort.
    pub fn received_master_abort(&self) -> bool {
        self.0.get_bit(13)
    }

    /// Will return `true`, by a master device, whenever its transaction is terminated with Target-Abort.
    pub fn received_target_abort(&self) -> bool {
        self.0.get_bit(12)
    }

    /// Will return `true` whenever a target device terminates a transaction with Target-Abort.
    pub fn signalled_target_abort(&self) -> bool {
        self.0.get_bit(11)
    }

    /// The slowest time that a device will assert DEVSEL# for any bus command except
    /// Configuration Space read and writes.
    ///
    /// For PCIe always set to `Fast`
    pub fn devsel_timing(&self) -> Result<DevselTiming> {
        let bits = self.0.get_bits(9..11);
        DevselTiming::try_from(bits as u8)
    }

    /// This returns `true` only when the following conditions are met:
    /// - The bus agent asserted PERR# on a read or observed an assertion of PERR# on a write
    /// - the agent setting the bit acted as the bus master for the operation in which the error occurred
    /// - bit 6 of the Command register (Parity Error Response bit) is set to 1.
    pub fn master_data_parity_error(&self) -> bool {
        self.0.get_bit(8)
    }

    /// If returns `true` the device can accept fast back-to-back transactions that are not from
    /// the same agent; otherwise, transactions can only be accepted from the same agent.
    ///
    /// For PCIe always set to `false`
    pub fn fast_back_to_back_capable(&self) -> bool {
        self.0.get_bit(7)
    }

    /// If returns `true` the device is capable of running at 66 MHz; otherwise, the device runs at 33 MHz.
    ///
    /// For PCIe always set to `false`
    pub fn capable_66mhz(&self) -> bool {
        self.0.get_bit(5)
    }

    /// If returns `true` the device implements the pointer for a New Capabilities Linked list;
    /// otherwise, the linked list is not available.
    ///
    /// For PCIe always set to `true`
    pub fn has_capability_list(&self) -> bool {
        self.0.get_bit(4)
    }

    /// Represents the state of the device's INTx# signal. If returns `true` and bit 10 of the
    /// Command register (Interrupt Disable bit) is set to 0 the signal will be asserted;
    /// otherwise, the signal will be ignored.
    pub fn interrupt_status(&self) -> bool {
        self.0.get_bit(3)
    }
}

impl core::fmt::Debug for PciStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PciStatus")
            .field("parity_error_detected", &self.parity_error_detected())
            .field("signalled_system_error", &self.signalled_system_error())
            .field("received_master_abort", &self.received_master_abort())
            .field("received_target_abort", &self.received_target_abort())
            .field("signalled_target_abort", &self.signalled_target_abort())
            .field("devsel_timing", &self.devsel_timing())
            .field("master_data_parity_error", &self.master_data_parity_error())
            .field(
                "fast_back_to_back_capable",
                &self.fast_back_to_back_capable(),
            )
            .field("capable_66mhz", &self.capable_66mhz())
            .field("has_capability_list", &self.has_capability_list())
            .field("interrupt_status", &self.interrupt_status())
            .finish()
    }
}

/// Slowest time that a device will assert DEVSEL# for any bus command except Configuration Space
/// read and writes
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevselTiming {
    Fast = 0x0,
    Medium = 0x1,
    Slow = 0x2,
}

impl TryFrom<u8> for DevselTiming {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x0 => Ok(DevselTiming::Fast),
            0x1 => Ok(DevselTiming::Medium),
            0x2 => Ok(DevselTiming::Slow),
            number => Err(Error::ParseFail(format!(
                "No DevselTiming for value{}",
                number
            ))),
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Command: u16 {
        const IO_ENABLE = 1 << 0;
        const MEMORY_ENABLE = 1 << 1;
        const BUS_MASTER_ENABLE = 1 << 2;
        const SPECIAL_CYCLE_ENABLE = 1 << 3;
        const MEMORY_WRITE_AND_INVALIDATE = 1 << 4;
        const VGA_PALETTE_SNOOP = 1 << 5;
        const PARITY_ERROR_RESPONSE = 1 << 6;
        const IDSEL_STEP_WAIT_CYCLE_CONTROL = 1 << 7;
        const SERR_ENABLE = 1 << 8;
        const FAST_BACK_TO_BACK_ENABLE = 1 << 9;
        const INTERRUPT_DISABLE = 1 << 10;
        const _ = !0;
    }
}
