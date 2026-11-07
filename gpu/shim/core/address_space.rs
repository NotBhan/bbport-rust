//! Address-space constants and permissions, ported from
//! `legacy/gpu/shim/core/address_space.h`. The VMA-backed operations live in the pending
//! `runtime_memory` port.

use std::ops::{BitOr, BitOrAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemoryPermission(pub u32);

impl MemoryPermission {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const READ_WRITE: Self = Self(3);
    pub const EXECUTE: Self = Self(4);
    pub const READ_WRITE_EXECUTE: Self = Self(7);
}

impl BitOr for MemoryPermission {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign for MemoryPermission {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AddressSpace;

impl AddressSpace {
    pub const SYSTEM_RESERVED_VIRTUAL_BASE: u64 = 0x7_0000_0000;
    pub const SYSTEM_RESERVED_VIRTUAL_SIZE: u64 = 0x1_0000_0000;
}
