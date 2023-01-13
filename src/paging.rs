use core::arch::asm;

use alloc::alloc::{alloc, Layout};
use bitflags::*;

bitflags! {
    pub struct PageDirectoryFlags: u32 {
        // https://wiki.osdev.org/Paging#Page_Directory
        const PRESENT = 0b1;
        const READ_WRITE = 0b10;
        const USER_SUPERVISOR = 0b100;
        const WRITE_THROUGH = 0b1000;
        const CACHE_DISABLED = 0b10_000;
        const ACCESSED = 0b100_000;
        const PAGE_SIZE = 0b10_000_000; // always 0
    }

    pub struct PageTableFlags: u32 {
        const PRESENT = 0b1;
        const READ_WRITE = 0b10;
        const USER_SUPERVISOR = 0b100;
        const WRITE_THROUGH = 0b1000;
        const CACHE_DISABLED = 0b10_000;
        const ACCESSED = 0b100_000;
        const DIRTY = 0b1_000_000;
        const PAGE_ATTRIBUTE_TABLE = 0b10_000_000;
        const GLOBAL = 0b100_000_000;
    }
}

pub fn addr_flags<T : BitFlags<Bits=u32>>(addr: u32, flags: T) -> u32 {
    addr | flags.bits()
}

pub unsafe fn create_page_table() -> &'static mut [u32] {
    const LEN: usize = 1024; // page tables are always 1024 elements long.
    let page_table = core::slice::from_raw_parts_mut(
        alloc(Layout::from_size_align_unchecked(LEN * 4, 4096)) as *mut u32,
        LEN
    );

    for i in 0..LEN {
        page_table[i] = addr_flags(i as u32 * 4096, PageTableFlags::READ_WRITE | PageTableFlags::PRESENT);
    }

    page_table
}

/// Creates a zeroed page directory with 1024 elements that's aligned to a 4096 boundary.
pub unsafe fn create_page_directory() -> &'static mut [u32] {
    const LEN: usize = 1024; // the page directory should always have 1024 elements.
    let page_dir = core::slice::from_raw_parts_mut(
        alloc(Layout::from_size_align_unchecked(LEN * 4, 4096)) as *mut u32,
        LEN
    );
    for i in 0..LEN {
        page_dir[i] = addr_flags(0, PageDirectoryFlags::READ_WRITE);
    }
    page_dir
}

pub unsafe fn enable(page_dir: *const u32) {
    asm!(
        "mov cr3, eax", // load the page directory address
        "mov eax, cr0",
        "or eax, 0x80000000", // set the paging bit
        "mov cr0, eax",
        in("eax") page_dir as u32,
        options(nomem, nostack)
    )
}
