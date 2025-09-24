use crate::addr_alloc::{AddressAllocator, AllocPolicy};

#[derive(Default)]
pub struct SimpleBarAllocator {
    mem32: Option<AddressAllocator>,
    mem64: Option<AddressAllocator>,
}

impl SimpleBarAllocator {
    pub fn set_mem32(&mut self, start: u32, size: u32) {
        self.mem32 = Some(AddressAllocator::new(start as _, size as _).unwrap());
    }

    pub fn set_mem64(&mut self, start: u64, size: u64) {
        self.mem64 = Some(AddressAllocator::new(start as _, size as _).unwrap());
    }

    pub fn alloc_memory32(&mut self, size: u32) -> Option<u32> {
        let res = self
            .mem32
            .as_mut()?
            .allocate(size as _, size as _, AllocPolicy::FirstMatch)
            .ok()?;
        Some(res.start() as _)
    }

    pub fn alloc_memory64(&mut self, size: u64) -> Option<u64> {
        let res = self
            .mem32
            .as_mut()?
            .allocate(size as _, size as _, AllocPolicy::FirstMatch)
            .ok()?;
        Some(res.start() as _)
    }
}

// trait Algin {
//     fn align_up(self, align: Self) -> Self;
// }

// impl Algin for u32 {
//     fn align_up(self, align: Self) -> Self {
//         (self + align - 1) & !(align - 1)
//     }
// }

// impl Algin for u64 {
//     fn align_up(self, align: Self) -> Self {
//         (self + align - 1) & !(align - 1)
//     }
// }
