use alloc::slice;
use bitfield::bitfield;
use bitflags::bitflags;
use spin::Mutex;
use core::{mem::{size_of, transmute}, arch::asm};

use crate::println;

bitfield! {
    pub struct Page(u32);
    u32;
    present, set_present: 0;
    rw, set_rw: 1;
    user, set_user: 2;
    accessed, set_accessed: 3;
    dirty, set_dirty: 4;
    unused, _: 5, 11;
    _frame_underlying, _: 12, 31;
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct PageFlags : u32 {
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
    const fn frame(&self) -> u32 { self.0.wrapping_shr(12) }
    fn set_frame(&mut self, value: u32) {
        // I stole this from C compiler generated assembly. Can't claim to understand how it works.
        self.0 = ((value & 0xFFFFF) << 12) | (self.0 & 0xFFF);
    }
}

extern "C" {
    static KERNEL_LOAD_ADDR: usize;
    static KERNEL_END_ADDR: usize; // we can safely allocate memory immediately after the end of the kernel
}
static mut PLACEMENT_ADDR: usize = 0x100_000;

// this exists because the actual allocation functions in heap.rs rely on paging.
unsafe fn kmalloc(size: usize, align: bool) -> *mut u8 {
    if align && PLACEMENT_ADDR & 0xFFFFF000 != 0 {
        // If not already aligned,
        // align it to the nearest page boundary (4K)
        PLACEMENT_ADDR = (PLACEMENT_ADDR & 0xFFFFF000) + 0x1000;
    }
    // this assumes nothing is ever freed, which is okay because this function is only called for 'static variables.
    let curr = PLACEMENT_ADDR;
    PLACEMENT_ADDR += size;
    curr as *mut u8
}

#[repr(transparent)]
pub struct PageTable {
    pages: [Page; 1024],
}

pub struct PageDirectory {
    page_tables: [*mut PageTable; 1024],
    physical_tables: [*const PageTable; 1024],
}

/// Maps every address between from and to to a random virtual address.
/// If identity is false, no guarantees are made about the virtual address.
unsafe fn map_addresses(dir: &mut PageDirectory, from: usize, to: &usize, identity: bool, flags: PageFlags) {
    // this can't be a for because to might change inside
    let mut i = from;
    while i < *to {
        set_page_frame(
            get_page(i, true, dir, flags).unwrap(),
            if identity { i / 0x1000 } else { get_free_frame() }
        );
        i += 0x1000;
    }
}

pub fn init() {
    let mem_end_page: usize = unsafe { &KERNEL_END_ADDR as *const usize as usize } + 0x100_000;
    let frames_num = mem_end_page / 0x1000;
    let arr_size = frames_num / 32;
    unsafe {
        // allocate an array with arr_size elements (each 4 bytes) at ptr and zero it
        let ptr = kmalloc(arr_size * 4, false);
        core::ptr::write_bytes(ptr, 0, arr_size * 4);
        *FRAMES_USAGE.lock() = slice::from_raw_parts_mut(ptr as *mut u32, arr_size);
    };

    // create a page directory for the kernel
    let kernel_dir: &mut PageDirectory = unsafe {
        // allocate it on the heap and zero it
        let ptr = kmalloc(size_of::<PageDirectory>(), true);
        core::ptr::write_bytes(ptr, 0, size_of::<PageDirectory>());
        transmute(ptr)
    };

    unsafe {
        // We need to identity map the first megabyte (the physical address should equal the virtual address)
        // This is because it is nearly all BIOS and stuff code that relies on being in certain addresses.
        // We also map everything that we `kmalloc`ed, since it comes directly afterwards anyways. (0x100_000..PLACEMENT_ADDR).
        map_addresses(
            kernel_dir,
            0,
            &PLACEMENT_ADDR,
            true,
            PageFlags::RW | PageFlags::USER
        );

        // Map the kernel addresses. They also need to be identity mapped since 
        // after paging is enabled, the IP stays the same, so it should point at the same code.
        map_addresses(
            kernel_dir,
            // the linker puts extern's values in their memory addresses.
            &KERNEL_LOAD_ADDR as *const usize as usize,
            &(&KERNEL_END_ADDR as *const usize as usize),
            true,
            PageFlags::RW | PageFlags::USER
        );

        // Actually enable paging CPU side
        switch_page_directory(kernel_dir);
    }
}

pub unsafe fn switch_page_directory(new: &PageDirectory) {
    asm!(
        "mov cr3, eax",
        "mov eax, cr0",
        "or eax, 0x80000000",
        "mov cr0, eax",
        in("eax") &new.physical_tables,
        options(nomem, nostack)
    );
}

/// Retrieves a reference to a page structure in the virtual memory.
/// If make is true, it will allocate a new page table in case the corresponding one is missing.
pub unsafe fn get_page(address: usize, make: bool, dir: &mut PageDirectory, flags: PageFlags) -> Option<&mut Page> {
    let index = address / 0x1000;
    // find the page table containing this index
    let table_idx = index / 1024;
    let table = dir.page_tables[table_idx];
    if !table.is_null() {
        Some(&mut (*table).pages[index % 1024])
    } else if make {
        // allocate a new page table
        let new_table = kmalloc(size_of::<PageTable>(), true) as *mut PageTable;

        dir.page_tables[table_idx] = new_table;
        core::ptr::write_bytes(new_table, 0, 0x1000);
        dir.physical_tables[table_idx] = (new_table as u32 | flags.bits() | PageFlags::PRESENT.bits()) as *const PageTable;

        Some(&mut (*new_table).pages[index % 1024])
    } else {
        None
    }
}

// an array in which each bit corresponds to whether its frame is used (1) or not (0)
static FRAMES_USAGE: Mutex<&mut [u32]> = Mutex::new(&mut []);

const fn get_idx_off(frame_idx: usize) -> (usize, usize) {
    (frame_idx / 32, frame_idx % 32) // 32: size of each element in the array
}

unsafe fn set_frame_used(frame_idx: usize, value: bool) {
    let mut usage = FRAMES_USAGE.lock();
    let (idx, off) = get_idx_off(frame_idx);
    if value {
        usage[idx] |= 0x1 << off;
    } else {
        usage[idx] &= !(0x1 << off);
    }
}

/// Returns the index of the first unused frame
pub fn get_free_frame() -> usize {
    let usage = FRAMES_USAGE.lock();
    // note: this is performance critical code. .into_iter().enumerate() is about 3 times slower.
    for i in 0..usage.len() {
        let frame = usage[i];
        // if every bit in frame is set, we don't need to check each bit individually
        if frame == 0xFFFFFFFF {
            continue;
        }
        for j in 0..32 {
            if frame & (0x1 << j) == 0 {
                return i * 32 + j;
            }
        }
    }
    panic!("All frames are used");
}

/// Sets the page's frame to frame
pub unsafe fn set_page_frame(page: &mut Page, frame: usize) {
    if page.frame() != 0 {
        panic!("Frame already linked!");
    }
    set_frame_used(frame, true);
    page.set_present(true);
    page.set_frame(frame as u32);
}

pub unsafe fn free_frame(page: &mut Page) {
    if page.frame() == 0 {
        return; // there's nothing to free!
    }
    set_frame_used(page.frame() as usize, false);
    page.set_frame(0);
}
