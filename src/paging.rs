use alloc::slice;
use bitfield::bitfield;
use bitflags::bitflags;
use spin::Mutex;
use core::{mem::{size_of, transmute, MaybeUninit}, arch::asm};

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
const HEAP_START: usize = 0x100_000;
static mut PLACEMENT_ADDR: usize = HEAP_START;
/// The end of the paging heap. Calculated after init()
static mut HEAP_END: usize = usize::MAX;

// this exists because the actual allocation functions in heap.rs rely on paging.
unsafe fn kmalloc(size: usize, align: bool) -> *mut u8 {
    if PLACEMENT_ADDR > HEAP_END {
        panic!("Out of paging-reserved memory!");
    }

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

/// Maps every address between phys_from and phys_to to a virtual address, starting at virt_addr.
pub unsafe fn map_addresses(dir: &mut PageDirectory, phys_from: usize, phys_to: usize, virt_addr: usize, identity: bool, flags: PageFlags) {
    // this can't be a for because to might change inside
    let mut i = 0;
    while i < phys_to - phys_from + 0xFFF {
        set_page_frame(
            get_page(virt_addr + i, true, dir, flags).unwrap(),
            if identity { (phys_from + i) / 0x1000 } else { get_free_frame() }
        );
        i += 0x1000;
    }
}

/// Initialises and enables paging. Returns the address at which the heap should begin
pub fn init() -> usize {
    let mem_end_page: usize = unsafe { &KERNEL_END_ADDR as *const _ as usize };
    let frames_num = mem_end_page / 0x1000;
    let arr_size = usize::div_ceil(frames_num, 32);
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
        // The only instances of using kmalloc after this are when allocating a single PageTable.
        // Theoretically there can only be 1024 of them, so
        HEAP_END = PLACEMENT_ADDR + size_of::<PageTable>() * 1024;
    }

    unsafe {
        // We need to identity map the first megabyte (the physical address should equal the virtual address)
        // This is because it is nearly all BIOS and stuff code that relies on being in certain addresses.
        // We also map everything that we `kmalloc`ed, since it comes directly afterwards anyways. (0x100_000..PLACEMENT_ADDR).
        map_addresses(
            kernel_dir,
            0,
            HEAP_END,
            0,
            true,
            PageFlags::RW | PageFlags::USER
        );

        // Map the kernel addresses. They also need to be identity mapped since 
        // after paging is enabled, the IP stays the same, so it should point at the same code.
        map_addresses(
            kernel_dir,
            // the linker puts extern's values in their memory addresses.
            &KERNEL_LOAD_ADDR as *const _ as usize,
            &KERNEL_END_ADDR as *const _ as usize,
            &KERNEL_LOAD_ADDR as *const _ as usize,
            true,
            PageFlags::RW | PageFlags::USER
        );

        // Actually enable paging CPU side
        switch_page_directory(kernel_dir);
    }

    // We've used the beginning of the "proper" heap with kmalloc, and we don't want to override anything.
    // Returning the first free address is the simplest solution.
    // We return the first free block (multiple of 4K) and not the first address to avoid double mapping
    unsafe {
        let x = HEAP_END + 1;
        x + 4096 - (x % 4096) + 4096
    }
}

pub unsafe fn switch_page_directory(new: &'static mut PageDirectory) {
    let tables_ptr = (&new.physical_tables) as *const _ as u32;
    CURR_DIR = MaybeUninit::new(new);
    asm!(
        "mov cr3, eax",
        "mov eax, cr0",
        "or eax, 0x80000000",
        "mov cr0, eax",
        in("eax") tables_ptr,
        options(nomem, nostack)
    );
}

/// Retrieves a reference to a page structure in the virtual memory.
/// If make is true, it will allocate a new page table in case the corresponding one is missing.
/// Flags only applicable if make is true
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

        let mut page = Page(0);
        page.set_user(flags.contains(PageFlags::USER));
        page.set_rw(flags.contains(PageFlags::RW));
        let page = page;

        dir.page_tables[table_idx] = new_table;
        for i in 0..(*new_table).pages.len() {
            (*new_table).pages[i] = page;
        }
        dir.physical_tables[table_idx] = (new_table as u32 | flags.bits() | PageFlags::PRESENT.bits()) as *const PageTable;

        let page = &mut (*new_table).pages[index % 1024];
        page.set_present(true);
        page.set_rw(flags.contains(PageFlags::RW));
        page.set_user(flags.contains(PageFlags::USER));
        Some(page)
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
        panic!("Page's frame already set!");
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

static mut CURR_DIR: MaybeUninit<&mut PageDirectory> = MaybeUninit::uninit();
pub fn default_directory() -> &'static mut PageDirectory {
    unsafe { CURR_DIR.assume_init_mut() }
}
