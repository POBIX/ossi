use bitfield::bitfield;
use bitflags::bitflags;

pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_ENTRIES: usize = 1024;

bitfield! {
    #[derive(Clone, Copy)]
    pub struct Page(u32);
    u32;
    pub present, set_present: 0;
    pub rw, set_rw: 1;
    pub user, set_user: 2;
    pub accessed, set_accessed: 3;
    pub dirty, set_dirty: 4;
    pub unused, _: 5, 11;
    _frame_underlying, _: 12, 31;
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct PageFlags : u32 {
        const NONE = 0;
        const PRESENT = 1;
        const RW = 2;
        const USER = 4;
        const PWT = 8;
        const PCD = 16;
        const ACCESSED = 32;
        const DIRTY = 64;
    }
}

impl Page {
    // we use custom implementations for frame because the default ones crash with overflows.
    pub const fn frame(&self) -> u32 { self.0.wrapping_shr(12) }
    pub fn set_frame(&mut self, value: u32) {
        // I stole this from C compiler generated assembly. Can't claim to understand how it works.
        self.0 = ((value & 0xFFFFF) << 12) | (self.0 & 0xFFF);
    }
}

#[repr(align(0x1000))] // 0x1000=PAGE_SIZE. Rust does not support constants in attributes.
pub struct PageTable {
    pub pages: [Page; PAGE_ENTRIES],
}

#[repr(C, align(0x1000))] // 0x1000=PAGE_SIZE. Rust does not support constants in attributes.
pub struct PageDirectory {
    pub page_tables: [u32; PAGE_ENTRIES],
}

