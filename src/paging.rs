use alloc::{boxed::Box};
use bitfield::bitfield;
use bitflags::bitflags;
use core::{mem::{size_of, transmute}, arch::asm};

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
pub(crate) static mut HEAP_END: usize = usize::MAX;

// this exists because the actual allocation functions in heap.rs rely on paging.
unsafe fn kmalloc(size: usize, align: bool) -> *mut u8 {
    if PLACEMENT_ADDR > HEAP_END {
        panic!("Out of paging-reserved memory!");
    }

    if align && PLACEMENT_ADDR & (PAGE_SIZE - 1) != 0 {
        // If not already aligned,
        // align it to the nearest page boundary 
        PLACEMENT_ADDR = (PLACEMENT_ADDR & !(PAGE_SIZE - 1)) + PAGE_SIZE;
    }
    // this assumes nothing is ever freed, which is okay because this function is only called for 'static variables.
    let curr = PLACEMENT_ADDR;
    PLACEMENT_ADDR += size;
    curr as *mut u8
}

static mut CURR_DIR: *mut PageDirectory = core::ptr::null_mut();
#[repr(align(0x1000))] // 0x1000=PAGE_SIZE. Rust does not support constants in attributes.
pub struct PageTable {
    pages: [Page; PAGE_ENTRIES],
}

#[repr(C, align(0x1000))] // 0x1000=PAGE_SIZE. Rust does not support constants in attributes.
pub struct PageDirectory {
    pub page_tables: [u32; PAGE_ENTRIES]
}

impl PageDirectory {
    /// Returns a new page directory, with the kernel mapped. Must be manually freed, via Box::from_raw
    pub fn new() -> *mut Self {
        unsafe {
            let dir: *mut Self = Box::into_raw(Box::new_zeroed().assume_init());
            (*dir).map_kernel(PageFlags::RW | PageFlags::USER);
            (*dir).map_self(PageFlags::RW | PageFlags::USER);
            (*dir).map_recursive(PageFlags::RW | PageFlags::USER);
            dir
        }
    }

    /// Identity maps the kernel addresses
    pub fn map_kernel(&mut self, flags: PageFlags) {
        unsafe {
            // We need to identity map the first megabyte (the physical address should equal the virtual address)
            // This is because it is nearly all BIOS and stuff code that relies on being in certain addresses.
            // We also map everything that we `kmalloc`ed, since it comes directly afterwards anyways. (0x100_000..PLACEMENT_ADDR).
            // We do NOT map the first page (0-PAGE_SIZE) since it includes null etc. and is not used
            self.identity_map(PAGE_SIZE, HEAP_END - PAGE_SIZE, flags);

            // Map the kernel addresses. They also need to be identity mapped since 
            // after paging is enabled, the IP stays the same, so it should point at the same code.
            let unaligned_size: usize = (&KERNEL_END_ADDR as *const _ as usize) - (&KERNEL_LOAD_ADDR as *const _ as usize);
            self.identity_map(
                &KERNEL_LOAD_ADDR as *const _ as usize, // the linker puts extern's values in their memory addresses.
                (unaligned_size + 0xFFF) & !0xFFF, // align the size (round up)
                flags
            );
        }
    }

    fn map_self(&mut self, flags: PageFlags) {
        unsafe {
            // map PageDirectory
            self.identity_map(
                self as *const _ as usize,
                size_of::<PageDirectory>(),
                flags
            );
        }
    }

    /// Maps the page directory's last table entry to the directory itself, so that it can be modified from within itself
    /// Weird trick that abuses the fact that page directories and page tables have the same bit structure
    fn map_recursive(&mut self, flags: PageFlags) {
        self.page_tables[self.page_tables.len() - 1] = (self as *const _ as u32) | (PageFlags::PRESENT | flags).bits();
        // We can now access the directory from within itself by referencing its last entry
        // i.e. the virtual address of each table is 0xFFC00000 + table_idx * PAGE_SIZE
        // and the virtual address of the page directory itself is 0xFFFFF000
    }

    /// Maps every virtual address from addr to addr+size to every physical address from addr to addr+size
    /// (Note: you probably want addr and size to be aligned)
    pub unsafe fn identity_map(&mut self, addr: usize, size: usize, flags: PageFlags) {
        for i in (0..size).step_by(PAGE_SIZE) {
            self.make_page(addr + i, addr + i, flags).unwrap_or_else(|_| panic!("{}", addr+i));
        }
    }

    /// Returns the virtual address of the directory, assuming a recursive map is set up in index 1023
    /// or if self != the currently active page directory, that self is mapped in index 1022
    fn get_dir_ptr(&mut self) -> *mut PageDirectory {
        if Self::curr().is_null() { // If paging is not yet enabled
            self
        } else if core::ptr::eq(self, Self::curr()) {
            0xFFFFF000 as *mut PageDirectory
        } else {
            0xFFBFE000 as *mut PageDirectory
        }
    }

    /// Returns the virtual address of a table, assuming a recursive map is set up in index 1023
    /// or if self != the currently active page directory, that self is mapped in index 1022
    fn get_table_ptr(&self, table_idx: usize) -> *mut PageTable {
        if Self::curr().is_null() { // If paging is not yet enabled
            (self.page_tables[table_idx] & 0xFFFFF000) as *mut PageTable
        } else {
            let offset: usize = if core::ptr::eq(self, Self::curr()) { 0xFFC00000 } else { 0xFF800000 };
            (offset + table_idx * PAGE_SIZE) as *mut PageTable
        }
    }

    /// Returns (the index of the page in the page table, the index of the table in the directory, the table), for the given address
    fn find_table(&mut self, address: usize) -> (usize, usize, *mut PageTable) {
        let index = address / PAGE_SIZE;
        let table_idx = index / PAGE_ENTRIES;

        let dir: *mut PageDirectory = self.get_dir_ptr();
        let virt: *mut PageTable = if unsafe {(*dir).page_tables}[table_idx] & PageFlags::PRESENT.bits() != 0 {
            self.get_table_ptr(table_idx)
        } else {
            core::ptr::null_mut()
        };

        (index % PAGE_ENTRIES, table_idx, virt)
    }

    /// Retrieves a reference to a page structure in the virtual memory.
    pub unsafe fn get_page(&mut self, address: usize) -> Option<&mut Page> {
        let (index, _, table) = self.find_table(address);
        if !table.is_null() {
            Some(&mut (*table).pages[index])
        } else {
            None
        }
    }

    /// Must be called on any change to the paging directory, or else the CPU will use the old cached value
    unsafe fn invalidate_tlb(virt_addr: usize) {
        asm!(
            "invlpg [{v}]",
            v = in(reg) virt_addr, 
            options(nostack, preserves_flags)
        );
    }

    /// Allocates a new page table at the specified index and returns its virtual address
    /// If self != Self::curr(), Self::curr()'s 1022nd page table must be set to self.
    unsafe fn make_table(&mut self, usage: &mut FramesUsage, index: usize, flags: PageFlags) -> *mut PageTable {
        // This function is called from within alloc::alloc. So we can't use that obviously
        let new_phys: u32 = if crate::heap::has_init() {
            // Thankfully a PageTable is precisely 4KB, so it fits perfectly inside of a frame. 
            let free_frame: usize = usage.get_free_frame();
            usage.set_frame_used(free_frame, true);
            (free_frame * PAGE_SIZE) as u32
        } else {
            // BUT if we're calling this during the directory initialisation, get_free_frame will return an unmapped address
            // We've reserved just enough memory for kmalloc to use such that we should never run out if we only ever call it
            // during initialisation indeed (kmalloc allocates in identity mapped memory)
            kmalloc(PAGE_SIZE, true) as u32
        };

        (*self.get_dir_ptr()).page_tables[index] = new_phys | PageFlags::PRESENT.bits() | flags.bits();

        let virt: *mut PageTable = self.get_table_ptr(index);

        // If we're modifying the active directory, we need to update the cache for the table to become mapped
        if core::ptr::eq(self, Self::curr()) {
            Self::invalidate_tlb(virt as usize); 
        } 

        core::ptr::write_bytes(virt, 0, 1); // Zero the page table

        virt
    }

    /// Creates a new page at virt_addr and maps it to phys_addr, or errors if that virtual address is already mapped
    /// (Does not check whether phys_addr is already used)
    pub unsafe fn make_page(&mut self, virt_addr: usize, phys_addr: usize, flags: PageFlags) -> Result<&mut Page, ()> {
        let mut usage = FRAMES_USAGE.lock();

        let in_kernel: bool = !Self::curr().is_null() && !core::ptr::eq(self, Self::curr());
        if in_kernel {
            // If we're modifying a different page directory from the active one,
            // then the page table that we're modifying won't be mapped. So we won't be able to write to it.
            // To combat this problem, we use a similar trick to map_recursive:
            let kernel: &mut PageDirectory = Self::curr().as_mut().unwrap();
            kernel.page_tables[1022] = (self as *const _ as u32) | (PageFlags::PRESENT | PageFlags::RW).bits();
            Self::invalidate_tlb(0xFF800000);
            Self::invalidate_tlb(0xFFBFE000);
            // We can now access the page table by 0xFF800000 + table_index * PAGE_SIZE.
        }

        let (index, table_idx, curr_table) = self.find_table(virt_addr);
        let table: *mut PageTable = // Allocate the table if it doesn't already exist
            if curr_table.is_null() { self.make_table(&mut usage, table_idx, flags) } 
            else { curr_table };

        // Assign the page (it already exists as it's been either zero initialised or used then freed)
        let page: &mut Page = &mut (*table).pages[index];
        if page.present() { return Err(()); }

        *page = Page(0); // Reset its state in case it's been freed
        page.set_user(flags.contains(PageFlags::USER));
        page.set_rw(flags.contains(PageFlags::RW));
        usage.set_page_frame(page, phys_addr / PAGE_SIZE);

        Self::invalidate_tlb(virt_addr); 

        if in_kernel {
            // We're done modifying, unmap it now
            let kernel: &mut PageDirectory = Self::curr().as_mut().unwrap();
            kernel.page_tables[1022] = 0;
            Self::invalidate_tlb(0xFF800000);
            Self::invalidate_tlb(0xFFBFE000);
        }

        Ok(page)
    }

    // Returns the first free virtual address, or None if none is available
    pub fn get_free_page(&mut self) -> Option<usize> {
        let tables: &[u32; PAGE_ENTRIES] = unsafe { &(*self.get_dir_ptr()).page_tables };
        for i in 0..PAGE_ENTRIES {
            let t = (tables[i] & 0xFFFFF000) as *mut PageTable;
            if t.is_null() {
                // If the table is unallocated, it means every page in it is free
                return Some(i * PAGE_ENTRIES * PAGE_SIZE);
            }

            for (j, page) in unsafe { (*t).pages }.iter().enumerate() {
                if !page.present() {
                    return Some((i * PAGE_ENTRIES + j) * PAGE_SIZE);
                }
            }
        }
        None
    }

    pub unsafe fn switch_to(&mut self) {
        CURR_DIR = self;
        asm!(
            "mov cr3, eax",
            in("eax") (&self.page_tables).as_ptr()
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
        let mut usage = FRAMES_USAGE.lock();
 
        // Don't free the last table, as it points at the directory (see map_recursive)
        for table in self.page_tables.iter().take(self.page_tables.len() - 1) {
            let t = (table & 0xFFFFF000) as usize; // physical address of the table

            // Free any table that was allocated on the actual heap (as opposed to the kernel heap)
            // (The kernel heap is a bump allocator and therefore freeing is useless)
            if t != 0 && t > unsafe { HEAP_END } {
                unsafe {
                    usage.set_frame_used(t / PAGE_SIZE, false);
                }
            }

            // Note: we don't free the memory *inside* of the tables since a single frame could
            // be mapped to several different directories (this happens for example with the kernel,
            // which every single directory maps).
            // Processes should free all memory they allocate....
        }
    }
}

// A struct in which each bit corresponds to whether its frame is used (1) or not (0)
#[repr(align(4))]
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
        for i in unsafe { (HEAP_END / PAGE_SIZE).div_ceil(32) }..self.0.len() { // We only start checking at HEAP_END since everything before is guaranteed to be in use
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
            panic!("Page's frame already set to {}!", page.frame());
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

pub(crate) static FRAMES_USAGE: spin::Mutex<FramesUsage> = spin::Mutex::new(FramesUsage([0; 32768]));

/// Initialises and enables paging. Returns the address at which the heap should begin
pub fn init() -> usize {
    // Create a page directory for the kernel.
    // PageDirectory::new cannot be used, nor can Box, since there's no allocator yet
    let kernel_dir: &mut PageDirectory = unsafe {
        // allocate it on the heap and zero it
        let ptr: *mut u8 = kmalloc(size_of::<PageDirectory>(), true);
        core::ptr::write_bytes(ptr, 0, size_of::<PageDirectory>());
        transmute(ptr)
    };

    unsafe {
        // The only instances of using kmalloc after this are when allocating a single PageTable.
        // Theoretically there can only be PAGE_ENTRIES of them, so
        HEAP_END = PLACEMENT_ADDR + size_of::<PageTable>() * PAGE_ENTRIES;
    }

    kernel_dir.map_kernel(PageFlags::RW);
    kernel_dir.map_recursive(PageFlags::RW);

    unsafe {
        // Activate the directory and actually enable paging CPU side
        kernel_dir.switch_to();
        asm!(
            "mov eax, cr0",
            "or eax, 0x80000000",
            "mov cr0, eax"
        );
    }

    // We've used the beginning of the "proper" heap with kmalloc, and we don't want to override anything.
    // Returning the first free address is the simplest solution.
    // We return the first free block (multiple of PAGE_SIZE) and not the first address to avoid double mapping
    unsafe {
        let x = HEAP_END + 1; // + 1 so that if it's already aligned to 4K we return the next block anyways
        HEAP_END = x + PAGE_SIZE - (x % PAGE_SIZE); // actually align it
        HEAP_END
    }
}
