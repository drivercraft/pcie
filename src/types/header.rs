use core::ptr::NonNull;

pub struct Header {
    pub common: PciHeaderCommon,
    pub kind: HeaderKind,
}

impl Header {
    pub fn read(data: NonNull<u8>) -> Self {
        unsafe {
            let common = data.cast::<PciHeaderCommon>().read_volatile();

            let kind_ptr = data.add(size_of::<PciHeaderCommon>());

            let kind = match common.header_type {
                HeaderType::Endpoint => HeaderKind::Endpoint(kind_ptr.cast().read_volatile()),
                HeaderType::PciBridge => HeaderKind::PciBridge(kind_ptr.cast().read_volatile()),
                HeaderType::Unknown(c) => HeaderKind::Unknown(c),
            };

            Self { common, kind }
        }
    }
}

pub enum HeaderKind {
    Endpoint(EndpointHeader),
    PciBridge(PciBridgeHeader),
    Unknown(u8),
}

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
#[repr(C)]
pub struct PciHeaderCommon {
    vendor_id: u16,
    device_id: u16,
    command: u16,
    status: u16,
    revision_id: u8,
    prog_if: u8,
    sub_class: u8,
    base_class: u8,
    cache_line_size: u8,
    latency_timer: u8,
    header_type: HeaderType,
    bist: u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderType {
    Endpoint,
    PciBridge,
    Unknown(u8),
}

/// Endpoints have a Type-0 header, so the remainder of the header is of the form:
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
///     +-----------------------------------------------------------+
///     |                  Base Address Register 2                  | 0x18
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 3                  | 0x1c
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 4                  | 0x20
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  Base Address Register 5                  | 0x24
///     |                                                           |
///     +-----------------------------------------------------------+
///     |                  CardBus CIS Pointer                      | 0x28
///     |                                                           |
///     +----------------------------+------------------------------+
///     |       Subsystem ID         |    Subsystem vendor ID       | 0x2c
///     |                            |                              |
///     +----------------------------+------------------------------+
///     |               Expansion ROM Base Address                  | 0x30
///     |                                                           |
///     +--------------------------------------------+--------------+
///     |                 Reserved                   | Capabilities | 0x34
///     |                                            |   Pointer    |
///     +--------------------------------------------+--------------+
///     |                         Reserved                          | 0x38
///     |                                                           |
///     +--------------+--------------+--------------+--------------+
///     |   Max_Lat    |   Min_Gnt    |  Interrupt   |  Interrupt   | 0x3c
///     |              |              |   pin        |   line       |
///     +--------------+--------------+--------------+--------------+
/// ```
#[repr(C)]
pub struct EndpointHeader {
    // 基地址寄存器 0
    pub base_address_register_0: u32, // 0x10

    // 基地址寄存器 1
    pub base_address_register_1: u32, // 0x14

    // 基地址寄存器 2
    pub base_address_register_2: u32, // 0x18

    // 基地址寄存器 3
    pub base_address_register_3: u32, // 0x1c

    // 基地址寄存器 4
    pub base_address_register_4: u32, // 0x20

    // 基地址寄存器 5
    pub base_address_register_5: u32, // 0x24

    // CardBus CIS 指针
    pub cardbus_cis_pointer: u32, // 0x28

    // 子系统 ID 和子系统供应商 ID
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,

    // 扩展 ROM 基地址
    pub expansion_rom_base_address: u32, // 0x30

    pub capabilities_pointer: u8, // 0x35
    pub reserved_1: [u8; 3],      // 0x36 - 0x37

    // 保留区域
    pub reserved_2: u32, // 0x38 - 0x3b

    // 最大延迟、最小授予时间、中断引脚和中断线
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub min_grant: u8,
    pub max_latency: u8,
}

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
pub struct PciBridgeHeader {
    // 基地址寄存器 0
    pub base_address_register_0: u32, // 0x10

    // 基地址寄存器 1
    pub base_address_register_1: u32, // 0x14

    // 次级延迟计时器、次级总线号、二级总线号、主总线号
    pub primary_bus_number: u8,
    pub secondary_bus_number: u8,
    pub subordinate_bus_number: u8,
    pub secondary_latency_timer: u8,

    // 次级状态、I/O 限制、I/O 基地址
    pub io_base: u8,
    pub io_limit: u8,
    pub secondary_status: u16,

    // 内存限制、内存基地址
    pub memory_base: u16,
    pub memory_limit: u16,

    // 可预取内存限制、可预取内存基地址
    pub prefetchable_memory_base: u16,
    pub prefetchable_memory_limit: u16,

    // 可预取基地址高 32 位
    pub prefetchable_base_upper_32_bits: u32, // 0x28

    // 可预取限制高 32 位
    pub prefetchable_limit_upper_32_bits: u32, // 0x2C

    // I/O 限制高 16 位、I/O 基地址高 16 位
    pub io_base_upper_16_bits: u16,
    pub io_limit_upper_16_bits: u16,

    // 保留、功能指针
    pub capability_pointer: u8,
    pub reserved: [u8; 3],

    // 扩展 ROM 基地址
    pub expansion_rom_base_address: u32, // 0x38

    // 桥接控制、中断引脚、中断线
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub bridge_control: u16,
}
