use bitfield::bitfield;
use bitflags::bitflags;

pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_ENTRIES: usize = 1024;

// The kernel needs to be able to modify other processes' page tables
// which is why we reserve the top part of virtual memory for page table allocation
// (If we allocated a page table in unmapped memory, and tried to modify it, we'd get a page fault)
pub(crate) const TABLES_SIZE: usize = 1;

// The top part of virtual memory for each diretory is reserved as a dynamic kernel heap.
// This is done because if the kernel allocates something, it has no way of informing processes
// that it's done so. So we have to keep it hardcoded.
pub(crate) const KERNEL_HEAP_SIZE: usize = 4; // 16MB
pub(crate) const KERNEL_HEAP_START: usize = PAGE_ENTRIES - TABLES_SIZE - KERNEL_HEAP_SIZE - 1;

pub const USER_MEM_FROM: usize = 1;
pub const USER_MEM_TO: usize = KERNEL_HEAP_START;

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

