use crate::{
    addr_alloc::{self, AddressAllocator, AllocPolicy},
    PciSpace32, PciSpace64,
};

#[derive(Default)]
pub struct SimpleBarAllocator {
    // Non-prefetchable windows
    mem32: Option<AddressAllocator>,
    mem64: Option<AddressAllocator>,
    // Prefetchable windows
    mem32_pref: Option<AddressAllocator>,
    mem64_pref: Option<AddressAllocator>,
}

impl SimpleBarAllocator {
    /// Convenience: add a 32-bit window with prefetchable attribute.
    pub fn set_mem32(&mut self, space: PciSpace32) -> Result<(), addr_alloc::Error> {
        let a = AddressAllocator::new(space.address as _, space.size as _)?;
        if space.prefetchable {
            self.mem32_pref = Some(a);
        } else {
            self.mem32 = Some(a);
        }
        Ok(())
    }

    /// Convenience: add a 64-bit window with prefetchable attribute.
    pub fn set_mem64(&mut self, space: PciSpace64) -> Result<(), addr_alloc::Error> {
        let a = AddressAllocator::new(space.address as _, space.size as _)?;
        if space.prefetchable {
            self.mem64_pref = Some(a);
        } else {
            self.mem64 = Some(a);
        }
        Ok(())
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
            .mem64
            .as_mut()?
            .allocate(size as _, size as _, AllocPolicy::FirstMatch)
            .ok()?;
        Some(res.start() as _)
    }

    /// Allocate from 32-bit windows considering prefetchable flag.
    pub fn alloc_memory32_with_pref(&mut self, size: u32, prefetchable: bool) -> Option<u32> {
        if prefetchable {
            if let Some(alloc) = self.mem32_pref.as_mut() {
                let res = alloc
                    .allocate(size as _, size as _, AllocPolicy::FirstMatch)
                    .ok()?;
                return Some(res.start() as _);
            }
        }
        // fallback to non-prefetchable window
        self.alloc_memory32(size)
    }

    /// Allocate from 64-bit windows considering prefetchable flag.
    pub fn alloc_memory64_with_pref(&mut self, size: u64, prefetchable: bool) -> Option<u64> {
        if prefetchable {
            if let Some(alloc) = self.mem64_pref.as_mut() {
                let res = alloc
                    .allocate(size as _, size as _, AllocPolicy::FirstMatch)
                    .ok()?;
                return Some(res.start() as _);
            }
        }
        // fallback to non-prefetchable window
        self.alloc_memory64(size)
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
