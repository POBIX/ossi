use alloc::slice;
use bitfield::bitfield;
use core::{mem::{size_of, transmute}, arch::asm};

bitfield! {
    pub struct Page(u32);
    u32;
    present, set_present: 0;
    rw, set_rw: 1;
    user, set_user: 2;
    accessed, set_accessed: 3;
    dirty, set_dirty: 4;
    unused, _: 5, 11;
    frame, set_frame: 12, 31;
}
extern "C" { 
    static KERNEL_LOAD_ADDR: usize;
    static KERNEL_END_ADDR: usize; // we can safely allocate memory immediately after the end of the kernel
}
static mut PLACEMENT_ADDR: usize = 0;

// this exists because the actual allocation functions in heap.rs rely on paging.
unsafe fn kmalloc(size: usize, align: bool, physical_addr: *mut usize) -> *mut u8 {
    if align && PLACEMENT_ADDR & 0xFFFFF000 != 0 {
        // If not already aligned,
        // align it to the nearest page boundary (4K)
        PLACEMENT_ADDR = (PLACEMENT_ADDR & 0xFFFFF000) + 0x1000;
    }
    if !physical_addr.is_null() {
        *physical_addr = PLACEMENT_ADDR;
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
    physical_tables: [u32; 1024],
    physical_address: u32,
}

pub fn init() {
    unsafe { PLACEMENT_ADDR = &KERNEL_END_ADDR as *const usize as usize; }
    const MEM_END_PAGE: usize = 0x1_000_000;
    let frames_num = MEM_END_PAGE / 0x1000;
    unsafe {
        // allocate an array with frames_num/32 elements at ptr and zero it
        let ptr = kmalloc(frames_num / 32, false, 0 as *mut usize);
        core::ptr::write_bytes(ptr, 0, frames_num / 32);
        FRAMES_USAGE = slice::from_raw_parts_mut(ptr as *mut u32, frames_num/32);
    };

    // create a page directory for the kernel
    let kernel_dir: &mut PageDirectory = unsafe {
        // allocate it on the heap and zero it
        let ptr = kmalloc(size_of::<PageDirectory>(), true, 0 as *mut usize);
        core::ptr::write_bytes(ptr, 0, size_of::<PageDirectory>());
        transmute(ptr)
    };

    // identity map the kernel code 
    // (the virtual address of everything in the kernel should be equal to its physical address)
    unsafe {
        let mut i = &KERNEL_LOAD_ADDR as *const usize as usize;
        // PLACEMENT_ADDR changes inside the loop, so this can't be a for loop
        while i < PLACEMENT_ADDR {
            alloc_frame(get_page(i, true, kernel_dir).unwrap());
            i += 0x1000;
        }

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

/// Retrieves a reference to a page structure in the virtual memory
pub unsafe fn get_page(address: usize, make: bool, dir: &mut PageDirectory) -> Option<&mut Page> {
    let index = address / 0x1000;
    // find the page table containing this index
    let table_idx = index / 1024;
    let table = dir.page_tables[table_idx];

    if !table.is_null() {
        Some(&mut (*table).pages[index % 1024])
    } else if make {
        // allocate a new page table
        let mut tmp: usize = 0;
        let new_table = kmalloc(size_of::<PageTable>(), true, &mut tmp as *mut usize) as *mut PageTable;

        dir.page_tables[table_idx] = new_table;
        core::ptr::write_bytes(new_table, 0, 0x1000);
        dir.physical_tables[table_idx] = tmp as u32 | 0x7; // PRESENT, RW, US

        Some(&mut (*new_table).pages[index % 1024])
    } else {
        None
    }
}

//TODO: mutex. This is a regular array for now just so it's easier to implement (the whole OS is single threaded anyways)
// an array in which each bit corresponds to whether its frame is used (1) or not (0)
static mut FRAMES_USAGE: &mut [u32] = &mut [];

const fn get_idx_off(frame_idx: usize) -> (usize, usize) {
    (frame_idx / 32, frame_idx % 32) // 32: size of each element in the array
}

unsafe fn set_frame_used(frame_idx: usize, value: bool) {
    let (idx, off) = get_idx_off(frame_idx);
    if value {
        FRAMES_USAGE[idx] |= 0x1 << off;
    } else {
        FRAMES_USAGE[idx] &= !(0x1 << off);
    }
}

fn is_frame_used(frame_idx: usize) -> bool {
    let (idx, off) = get_idx_off(frame_idx);
    unsafe { FRAMES_USAGE[idx] & 0x1 << off != 0 }
}

/// Returns the index of the first unused frame
pub fn get_free_frame() -> usize {
    unsafe {
        // note: this is performance critical code. .into_iter().enumerate() is about 3 times slower.
        for i in 0..FRAMES_USAGE.len() {
            let frame = FRAMES_USAGE[i];
            // if every bet in frame is set, we don't need to check each bit individually
            if frame == 0xFFFFFFFF {
                continue;
            }
            for j in 0..32 {
                if frame & (0x1 << j) == 0 {
                    return j * 32 + i;
                }
            }
        }
    }
    panic!("All frames are used");
}

/// Allocates a frame and links it to page
pub unsafe fn alloc_frame(page: &mut Page) {
    if page.frame() != 0 {
        return; // the frame was already allocated, we don't need to do anything
    }
    // we get a free frame, mark it (and the page) as used, and link it to the page
    let idx = get_free_frame();
    set_frame_used(idx, true);
    page.set_present(true);
    page.set_frame(idx as u32);
}

pub unsafe fn free_frame(page: &mut Page) {
    if page.frame() == 0 {
        return; // there's nothing to free!
    }
    set_frame_used(page.frame() as usize, false);
    page.set_frame(0);
}
