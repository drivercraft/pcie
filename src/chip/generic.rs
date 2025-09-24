use core::ptr::NonNull;

use crate::root::RootComplex;
use core::ops::{Deref, DerefMut};

use super::{PcieController, PcieGeric};

// 保留原有 Generic 名称作为语义占位，以减少外部改动。
pub struct Generic;

// 兼容旧 API：提供 RootComplexGeneric 包装器，保留 `new(mmio_base)` 构造并透传方法。
pub struct RootComplexGeneric(pub RootComplex);

impl RootComplexGeneric {
    // 旧 API：通过 mmio_base 创建一个通用控制器，再包装成 RootComplex
    pub fn new(mmio_base: NonNull<u8>) -> Self {
        let ctrl = PcieController::new(PcieGeric::new(mmio_base));
        RootComplexGeneric(RootComplex::new(ctrl))
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
