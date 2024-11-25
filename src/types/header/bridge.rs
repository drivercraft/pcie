use crate::Chip;

use super::Header;

/// PCI-PCI Bridges have a Type-1 header, so the remainder of the header is of the form:
/// ```ignore
///     32                           16                              0
///     +-----------------------------------------------------------+ 0x00
///     |                                                           |
///     |                Predefined region of header                |
///     |                                                           |
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 0                  | 0x10
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 1                  | 0x14
///     |                                                           |
///     +--------------+--------------+--------------+--------------+
///     | Secondary    | Subordinate  |  Secondary   | Primary Bus  | 0x18
///     |Latency Timer | Bus Number   |  Bus Number  |   Number     |
///     +--------------+--------------+--------------+--------------+
///     |      Secondary Status       |  I/O Limit   |   I/O Base   | 0x1C
///     |                             |              |              |
///     +-----------------------------+--------------+--------------+
///     |        Memory Limit         |         Memory Base         | 0x20
///     |                             |                             |
///     +-----------------------------+-----------------------------+
///     |  Prefetchable Memory Limit  |  Prefetchable Memory Base   | 0x24
///     |                             |                             |
///     +-----------------------------+-----------------------------+
///     |             Prefetchable Base Upper 32 Bits               | 0x28
///     |                                                           |
///     +-----------------------------------------------------------+
///     |             Prefetchable Limit Upper 32 Bits              | 0x2C
///     |                                                           |
///     +-----------------------------+-----------------------------+
///     |   I/O Limit Upper 16 Bits   |   I/O Base Upper 16 Bits    | 0x30
///     |                             |                             |
///     +-----------------------------+--------------+--------------+
///     |              Reserved                      |  Capability  | 0x34
///     |                                            |   Pointer    |
///     +--------------------------------------------+--------------+
///     |                  Expansion ROM base address               | 0x38
///     |                                                           |
///     +-----------------------------+--------------+--------------+
///     |    Bridge Control           |  Interrupt   | Interrupt    | 0x3C
///     |                             |     PIN      |   Line       |
///     +-----------------------------+--------------+--------------+
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PciBridge {
}


