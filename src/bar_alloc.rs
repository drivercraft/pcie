use core::ops::Range;

use crate::BarAllocator;

#[derive(Default)]
pub struct SimpleBarAllocator {
    mem32: Range<u32>,
    mem32_iter: u32,
    mem64: Range<u64>,
    mem64_iter: u64,
}

impl SimpleBarAllocator {
    pub fn new(mem32_start: u32, mem32_size: u32, mem64_start: u64, mem64_size: u64) -> Self {
        Self {
            mem32: mem32_start..mem32_start + mem32_size,
            mem64: mem64_start..mem64_start + mem64_size,
            mem32_iter: mem32_start,
            mem64_iter: mem64_start,
        }
    }

    pub fn set_mem32(&mut self, start: u32, size: u32) {
        self.mem32 = start..start + size;
        self.mem32_iter = start;
    }

    pub fn set_mem64(&mut self, start: u64, size: u64) {
        self.mem64 = start..start + size;
        self.mem64_iter = start;
    }
}

impl BarAllocator for SimpleBarAllocator {
    fn alloc_memory32(&mut self, size: u32) -> Option<u32> {
        let ptr = self.mem32_iter.align_up(size);

        if self.mem32.contains(&ptr) && ptr + size <= self.mem32.end {
            self.mem32_iter = ptr + size;
            Some(ptr)
        } else {
            None
        }
    }

    fn alloc_memory64(&mut self, size: u64) -> Option<u64> {
        let ptr = self.mem64_iter.align_up(size);
        if self.mem64.contains(&ptr) && ptr + size <= self.mem64.end {
            self.mem64_iter = ptr + size;
            Some(ptr)
        } else {
            None
        }
    }
}

trait Algin {
    fn align_up(self, align: Self) -> Self;
}

impl Algin for u32 {
    fn align_up(self, align: Self) -> Self {
        (self + align - 1) & !(align - 1)
    }
}

impl Algin for u64 {
    fn align_up(self, align: Self) -> Self {
        (self + align - 1) & !(align - 1)
    }
}
