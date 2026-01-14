use alloc::{boxed::Box};
use bitfield::bitfield;
use bitflags::bitflags;
use core::{mem::{size_of, transmute}, arch::asm};

pub const PAGE_SIZE: usize = 0x1000;

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
    pages: [Page; 1024],
}

#[repr(C, align(0x1000))] // 0x1000=PAGE_SIZE. Rust does not support constants in attributes.
pub struct PageDirectory {
    pub page_tables: [*mut PageTable; 1024]
}

impl PageDirectory {
    /// Returns a new page directory, with the kernel mapped. Must be manually freed, via Box::from_raw
    pub fn new() -> *mut Self {
        unsafe {
            let dir: *mut Self = Box::into_raw(Box::new_zeroed().assume_init());
            (*dir).map_kernel();
            (*dir).map_self();
            dir
        }
    }

    /// Identity maps the kernel addresses
    pub fn map_kernel(&mut self) {
        unsafe {
            // We need to identity map the first megabyte (the physical address should equal the virtual address)
            // This is because it is nearly all BIOS and stuff code that relies on being in certain addresses.
            // We also map everything that we `kmalloc`ed, since it comes directly afterwards anyways. (0x100_000..PLACEMENT_ADDR).
            self.identity_map(
                0,
                HEAP_END,
                PageFlags::RW | PageFlags::USER
            );

            // Map the kernel addresses. They also need to be identity mapped since 
            // after paging is enabled, the IP stays the same, so it should point at the same code.
            self.identity_map(
                // the linker puts extern's values in their memory addresses.
                &KERNEL_LOAD_ADDR as *const _ as usize,
                (&KERNEL_END_ADDR as *const _ as usize) - (&KERNEL_LOAD_ADDR as *const _ as usize),
                PageFlags::RW | PageFlags::USER
            );
        }
    }

    fn map_self(&mut self) {
        unsafe {
            // map PageDirectory
            self.identity_map(
                self as *const _ as usize,
                size_of::<PageDirectory>(),
                PageFlags::RW | PageFlags::USER
            );
        }
        // Maps the page directory's last table entry to the directory itself, so that it can be modified from within itself
        // Weird trick that abuses the fact that page directories and page tables have the same bit structure
        self.page_tables[self.page_tables.len() - 1] = 
            ((self as *const _ as u32) | PageFlags::PRESENT.bits() | PageFlags::RW.bits()) as *mut PageTable;
        // We can now access the directory from within itself by referencing its last entry
        // i.e. the virtual address of each table is 0xFFC00000 + whatever
    }

    /// Maps every virtual address from addr to addr+size to every physical address from addr to addr+size
    pub unsafe fn identity_map(&mut self, addr: usize, size: usize, flags: PageFlags) {
        let usage = FRAMES_USAGE.lock();
        for i in (0..size).step_by(PAGE_SIZE) {
            usage.set_page_frame(
                self.make_page(addr + i, flags, true).unwrap(),
                (addr + i) / PAGE_SIZE
            );
        }
    }

    /// Returns (the index of the page in the page table, the index of the table in the directory, the table), for the given address
    fn find_table(&self, address: usize) -> (usize, usize, *mut PageTable) {
        let index = address / PAGE_SIZE;
        let table_idx = index / self.page_tables.len();
        (index % self.page_tables.len(), table_idx, self.page_tables[table_idx])
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

    /// Creates a new page at address, or errors if that address already has a page
    pub unsafe fn make_page(&mut self, address: usize, flags: PageFlags, kernel: bool) -> Result<&mut Page, ()> {
        let mut usage = FRAMES_USAGE.lock();

        let (index, table_idx, curr_table) = self.find_table(address);
        let table = if curr_table.is_null() { 
            // Nonexistent table, we need to allocate it
            // This function is called from within alloc::alloc. So we can't use that obviously
            let new_table: *mut PageTable = if !kernel {
                // Thankfully a PageTable is precisely 4KB, so it fits perfectly inside of a frame. 
                let free_frame: usize = usage.get_free_frame();
                usage.set_frame_used(free_frame, true);
                // virtual memory offset (see map_self) + physical address of the frame
                (0xFFC00000 + free_frame * PAGE_SIZE) as *mut PageTable
            } else {
                // BUT if we're calling this during the directory initialisation, get_free_frame will return an unmapped address
                // We've reserved just enough memory for kmalloc to use such that we should never run out if we only ever call it
                // during initialisation indeed
                kmalloc(PAGE_SIZE, true) as *mut PageTable
            };
            self.page_tables[table_idx] = new_table;
            core::ptr::write_bytes(new_table, 0, 1); // zero initialise the table
            new_table
        } else {
            curr_table
        };
 
        // Assign the page (it already exists as it's been either zero initialised or used then freed)
        let page: &mut Page = &mut (*table).pages[index];
        if page.present() {
            return Err(());
        }
        *page = Page(0); // Reset its state in case it's been freed
        page.set_user(flags.contains(PageFlags::USER));
        page.set_rw(flags.contains(PageFlags::RW));

        // Assign it (let the computer know about it)
        usage.set_page_frame(page, usage.get_free_frame());

        Ok(page)
    }

    pub unsafe fn switch_to(&mut self) {
        let tables_ptr = (&self.physical_tables).as_ptr();
        CURR_DIR = self;
        asm!(
            "mov cr3, eax",
            in("eax") tables_ptr
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
        let usage = FRAMES_USAGE.lock();
        for table in self.page_tables {
            let t = table as usize;
            // If this table was allocated on the actual heap (rather than the kernel heap)
            if !table.is_null() && (t < HEAP_START || t > unsafe { HEAP_END }) {
                unsafe {
                    usage.set_frame_used((t - 0xFFC00000) / PAGE_SIZE, false);
                }
            }
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
    *FRAMES_USAGE.get_mut() = unsafe {
        // allocate an array with arr_size elements (each 4 bytes) at ptr and zero it
        let ptr: *mut u8 = kmalloc(size_of::<FramesUsage>(), false);
        core::ptr::write_bytes(ptr, 0, size_of::<FramesUsage>());
        *(ptr as *mut FramesUsage)
    };

    unsafe {
        // The only instances of using kmalloc after this are when allocating a single PageTable.
        // Theoretically there can only be 1024 of them, so
        HEAP_END = PLACEMENT_ADDR + size_of::<PageTable>() * 1024;
    }

    kernel_dir.map_kernel();

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
        let x = HEAP_END + 1;
        x + PAGE_SIZE - (x % PAGE_SIZE) + PAGE_SIZE
    }
}
