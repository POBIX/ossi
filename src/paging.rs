use alloc::boxed::Box;
use bitfield::bitfield;
use bitflags::bitflags;
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

static mut CURR_DIR: *mut PageDirectory = core::ptr::null_mut();
#[repr(transparent)]
pub struct PageTable {
    pages: [Page; 1024],
}

#[repr(C)]
pub struct PageDirectory {
    page_tables: [*mut PageTable; 1024],
    physical_tables: [*const PageTable; 1024],
    frames_usage: *mut FramesUsage
}

impl PageDirectory {
    /// Returns a new page directory, with the kernel mapped. Must be manually freed, via Box::from_raw
    pub fn new() -> *mut Self {
        unsafe {
            let dir: *mut Self = Box::into_raw(Box::new_zeroed().assume_init());
            (*dir).frames_usage = Box::into_raw(Box::new_zeroed().assume_init());
            (*dir).map_kernel();
            dir
        }
    }

    /// Identity maps the kernel addresses
    pub fn map_kernel(&mut self) {
        unsafe {
            // We need to identity map the first megabyte (the physical address should equal the virtual address)
            // This is because it is nearly all BIOS and stuff code that relies on being in certain addresses.
            // We also map everything that we `kmalloc`ed, since it comes directly afterwards anyways. (0x100_000..PLACEMENT_ADDR).
            self.map_addresses(
                0,
                HEAP_END,
                0,
                PageFlags::RW | PageFlags::USER
            );

            // Map the kernel addresses. They also need to be identity mapped since 
            // after paging is enabled, the IP stays the same, so it should point at the same code.
            self.map_addresses(
                // the linker puts extern's values in their memory addresses.
                &KERNEL_LOAD_ADDR as *const _ as usize,
                &KERNEL_END_ADDR as *const _ as usize,
                &KERNEL_LOAD_ADDR as *const _ as usize,
                PageFlags::RW | PageFlags::USER
            );
        }
    }

    /// Maps every address between from and to to a range starting at virt_addr
    pub unsafe fn map_addresses(&mut self, from: usize, to: usize, virt_addr: usize, flags: PageFlags) {
        // this can't be a for because to might change inside
        let mut i = 0;
        while i < to - from {
            (*self.frames_usage).set_page_frame(
                self.get_page(virt_addr + i, true, flags).unwrap(),
                (from + i) / 0x1000
            );
            i += 0x1000;
        }
    }

    /// Retrieves a reference to a page structure in the virtual memory.
    /// If make is true, it will allocate a new page table in case the corresponding one is missing.
    /// Flags only applicable if make is true
    pub unsafe fn get_page(&mut self, address: usize, make: bool, flags: PageFlags) -> Option<&mut Page> {
        let index = address / 0x1000;
        // find the page table containing this index
        let table_idx = index / 1024;
        let table = self.page_tables[table_idx];
        if !table.is_null() {
            Some(&mut (*table).pages[index % 1024])
        } else if make {
            // allocate a new page table
            let new_table = kmalloc(size_of::<PageTable>(), true) as *mut PageTable;

            let mut page = Page(0);
            page.set_user(flags.contains(PageFlags::USER));
            page.set_rw(flags.contains(PageFlags::RW));
            let page = page;

            self.page_tables[table_idx] = new_table;
            for i in 0..(*new_table).pages.len() {
                (*new_table).pages[i] = page;
            }
            self.physical_tables[table_idx] = (new_table as u32 | flags.bits() | PageFlags::PRESENT.bits()) as *const PageTable;

            let page = &mut (*new_table).pages[index % 1024];
            page.set_present(true);
            page.set_rw(flags.contains(PageFlags::RW));
            page.set_user(flags.contains(PageFlags::USER));
            Some(page)
        } else {
            None
        }
    }

    pub unsafe fn switch_to(&mut self) {
        let tables_ptr = (&self.physical_tables) as *const _ as u32;
        CURR_DIR = self;
        asm!(
            "mov cr3, eax",
            "mov eax, cr0",
            "or eax, 0x80000000",
            "mov cr0, eax",
            in("eax") tables_ptr,
            options(nostack)
        );
    }

    // Not marking this function unsafe since there will always be an active page directory (excluding the init() function).
    // If you freed a page directory while it was in use your code is going to crash immediately anyways.
    pub fn curr() -> *mut PageDirectory {
        unsafe { CURR_DIR }
    }
}

impl Drop for PageDirectory {
    fn drop(&mut self) {
        unsafe { core::mem::drop(Box::from_raw(self.frames_usage)); }
    }
}


// A struct in which each bit corresponds to whether its frame is used (1) or not (0)
#[repr(transparent)]
pub struct FramesUsage([u32; 32768]); // 32768 - covers all possible addresses for 32bit

impl FramesUsage {
    const fn get_idx_off(frame_idx: usize) -> (usize, usize) {
        (frame_idx / 32, frame_idx % 32) // 32: size of each element in the array
    }

    unsafe fn set_frame_used(&mut self, frame_idx: usize, value: bool) {
        let (idx, off) = Self::get_idx_off(frame_idx);
        if value {
            self.0[idx] |= 0x1 << off;
        } else {
            self.0[idx] &= !(0x1 << off);
        }
    }

    /// Returns the index of the first unused frame
    pub fn get_free_frame(&self) -> usize {
        // note: this is performance critical code. .into_iter().enumerate() is about 3 times slower.
        for i in 0..self.0.len() {
            let frame = self.0[i];
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
    pub unsafe fn set_page_frame(&mut self, page: &mut Page, frame: usize) {
        if page.frame() != 0 {
            panic!("Page's frame already set!");
        }
        self.set_frame_used(frame, true);
        page.set_present(true);
        page.set_frame(frame as u32);
    }

    pub unsafe fn free_frame(&mut self, page: &mut Page) {
        if page.frame() == 0 {
            return; // there's nothing to free!
        }
        self.set_frame_used(page.frame() as usize, false);
        page.set_frame(0);
    }
}



/// Initialises and enables paging. Returns the address at which the heap should begin
pub fn init() -> usize {
    // Create a page directory for the kernel.
    // PageDirectory::new cannot be used, nor can Box, since there's no allocator yet
    let kernel_dir: &mut PageDirectory = unsafe {
        // allocate it on the heap and zero it
        let ptr = kmalloc(size_of::<PageDirectory>(), true);
        core::ptr::write_bytes(ptr, 0, size_of::<PageDirectory>());
        transmute(ptr)
    };
    kernel_dir.frames_usage = unsafe {
        // allocate an array with arr_size elements (each 4 bytes) at ptr and zero it
        let ptr = kmalloc(size_of::<FramesUsage>(), false);
        core::ptr::write_bytes(ptr, 0, size_of::<FramesUsage>());
        &mut *(ptr as *mut FramesUsage)
    };

    unsafe {
        // The only instances of using kmalloc after this are when allocating a single PageTable.
        // Theoretically there can only be 1024 of them, so
        HEAP_END = PLACEMENT_ADDR + size_of::<PageTable>() * 1024;
    }

    kernel_dir.map_kernel();

    unsafe {
        // Actually enable paging CPU side
        kernel_dir.switch_to();
    }

    // We've used the beginning of the "proper" heap with kmalloc, and we don't want to override anything.
    // Returning the first free address is the simplest solution.
    // We return the first free block (multiple of 4K) and not the first address to avoid double mapping
    unsafe {
        let x = HEAP_END + 1;
        x + 4096 - (x % 4096) + 4096
    }
}
