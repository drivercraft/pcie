mod header;

pub use header::*;

pub type VendorId = u16;
pub type DeviceId = u16;
pub type DeviceRevision = u8;
pub type BaseClass = u8;
pub type SubClass = u8;
pub type Interface = u8;
pub type SubsystemId = u16;
pub type SubsystemVendorId = u16;
pub type InterruptLine = u8;
pub type InterruptPin = u8;
