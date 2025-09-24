use core::ptr::NonNull;

use crate::root::RootComplex;
use core::ops::{Deref, DerefMut};

use super::{PcieController, PcieGeneric};

// 保留原有 Generic 名称作为语义占位，以减少外部改动。
pub struct Generic;

// 兼容旧 API：提供 RootComplexGeneric 包装器，保留 `new(mmio_base)` 构造并透传方法。
pub struct RootComplexGeneric(pub RootComplex);

impl RootComplexGeneric {
    // 旧 API：通过 mmio_base 创建一个通用控制器，再包装成 RootComplex
    pub fn new(mmio_base: NonNull<u8>) -> Self {
        let ctrl = PcieController::new(PcieGeneric::new(mmio_base));
        // 默认不带 allocator，需由调用者设置或通过 new_with_spaces 构造
        RootComplexGeneric(RootComplex::new(ctrl, None, None))
    }

    pub fn new_with_spaces(
        mmio_base: NonNull<u8>,
        space32: Option<crate::PciSpace32>,
        space64: Option<crate::PciSpace64>,
    ) -> Self {
        let ctrl = PcieController::new(PcieGeneric::new(mmio_base));
        RootComplexGeneric(RootComplex::new(ctrl, space32, space64))
    }

    pub fn set_allocator(&mut self, alloc: crate::SimpleBarAllocator) {
        self.0.set_allocator(alloc);
    }
}

impl Deref for RootComplexGeneric {
    type Target = RootComplex;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RootComplexGeneric {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
