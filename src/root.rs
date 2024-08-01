use crate::{preludes::*, types::*, PciDevice};
use alloc::vec::Vec;
use log::*;

const MAX_DEVICE: u8 = 31;
const MAX_FUNCTION: u8 = 7;

/// The root complex of a PCI bus.
#[derive(Clone)]
pub struct RootComplex<C: Chip> {
    chip: C,
}

impl<C: Chip> RootComplex<C> {
    pub fn new(mmio_base: usize) -> Self {
        Self {
            chip: C::new(mmio_base),
        }
    }

    /// Enumerates PCI devices on the given bus.
    pub fn enumerate(&self) -> BusDeviceIterator<C> {
        // Safe because the BusDeviceIterator only reads read-only fields.
        BusDeviceIterator {
            root: self.clone(),
            segment: 0,
            bus: 0,
            device: 0,
            function: 0,
            stack: Vec::new(),
            bus_iter: 0,
        }
    }
}

pub struct BusDeviceIterator<C: Chip> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: RootComplex<C>,
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
    stack: Vec<PciPciBridgeHeader>,
    bus_iter: u8,
}

impl<C: Chip> BusDeviceIterator<C> {
    fn access(&self) -> &C {
        &self.root.chip
    }

    fn current(&self) -> PciAddress {
        PciAddress::new(self.segment, self.bus, self.device, self.function)
    }
}

impl<C: Chip> Iterator for BusDeviceIterator<C> {
    type Item = PciDevice<C>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.function > MAX_FUNCTION {
                self.function = 0;
                self.device += 1;
            }
            if self.device > MAX_DEVICE {
                if let Some(parent) = self.stack.pop() {
                    // parent.set_subordinate_bus_number(self.bus_iter, self.access());
                    parent.update_bus_number(self.access(), |mut bus| {
                        bus.subordinate = self.bus_iter;
                        bus
                    });

                    self.bus = parent.header().address().bus();
                    self.device = parent.header().address().device() + 1;
                    self.function = 0;

                    trace!(
                        "{:?} Bridge set primary bus: {}, secondary bus: {}, subordinate bus: {}",
                        parent.header().address(),
                        parent.primary_bus_number(self.access()),
                        parent.secondary_bus_number(self.access()),
                        parent.subordinate_bus_number(self.access()),
                    );

                    continue;
                } else {
                    return None;
                }
            }
            let current = self.current();
            let header = PciHeader::new(current);

            let (vendor_id, _) = header.id(self.access());

            if vendor_id == 0xffff {
                if current.function() == 0 {
                    self.device += 1;
                } else {
                    self.function += 1;
                }
                continue;
            }
            let device = PciDevice::new(self.root.chip.clone(), &header);
            let multi = header.has_multiple_functions(self.access());
            match &device {
                PciDevice::PciPciBridge(bridge) => {
                    self.bus_iter += 1;
                    let primary = self.bus;
                    let secondary = self.bus_iter;
                    bridge.header.update_bus_number(self.access(), |mut bus| {
                        bus.primary = primary;
                        bus.secondary = secondary;
                        bus.subordinate = 0xFF;
                        bus
                    });
                    self.bus = self.bus_iter;
                    self.stack
                        .push(PciPciBridgeHeader::from_header(header, self.access()).unwrap());
                    self.device = 0;
                    self.function = 0;
                }
                _ => {
                    if current.function() == 0 && !multi {
                        self.device += 1;
                    } else {
                        self.function += 1;
                    }
                }
            }
            return Some(device);
        }
    }
}
