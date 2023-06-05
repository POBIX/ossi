use core::mem::size_of;

use alloc::alloc::{GlobalAlloc, Layout};

use crate::paging::{self, PageFlags};
/// struct that gets placed in memory every time an allocation occurs,
/// informing the allocator whether the data can be overridden and
/// how much of it there is. immediately after the struct comes
/// the actual user data, and immediately after size bytes of user data - another Data struct.
#[repr(packed)]
#[derive(Copy, Clone)]
struct Data {
    /// can this data be overridden?
    used: bool,
    /// size of the data in bytes
    size: usize
}

pub struct Heap {
    memory: *mut u8,
    size: usize,
}

impl Heap {
    /// allocates memory without doing any checks whatsoever.
    #[inline]
    unsafe fn alloc_raw(&self, ptr: *mut u8, size: usize, used: bool) -> *mut u8 {
        *(ptr as *mut Data) = Data { used, size };
        ptr.add(size_of::<Data>()) // address of actual user data (without the struct)
    }

    /// reallocates previously deallocated memory and creates a new unused block in the remaining space
    unsafe fn alloc_chunk(&self, ptr: *mut u8, alloc_size: usize, block_size: usize) -> *mut u8 {
        let data = ptr as *mut Data;

        let remaining_space = block_size - alloc_size;

        // if there isn't enough space left to create another block, simply mark the existing one as used
        // and return the pointer, with a little bit of leftover space.
        if remaining_space <= size_of::<Data>() {
            (*data).used = true;
            // (*data).size = block_size;
            return ptr.add(size_of::<Data>());
        }

        self.alloc_raw(ptr.add(size_of::<Data>() + alloc_size), remaining_space - size_of::<Data>(), false);
        self.alloc_raw(ptr, alloc_size, true)
    }

    /// this function should be called to check whether there is enough available memory
    /// if data.size==0 (the last allocated block of memory).
    /// panics if out of memory, returns the amount of available memory otherwise.
    #[inline]
    unsafe fn panic_if_oom(&self, ptr: *const u8, size: usize) -> usize {
        // rem = self.memory + self.size - (ptr + size)
        let rem = (self.memory.add(self.size) as i64) - (ptr.add(size) as i64);
        if rem <= 0 {
            panic!("OOM: tried to allocate {} bytes, ran out of memory by {} bytes!", size, rem.abs())
        }
        rem.try_into().unwrap()
    }

    pub(crate) unsafe fn alloc_internal(&self, layout: Layout) -> *mut u8 {
        crate::interrupts::disable(); // prevent two simultaneous allocations

        // iterate over our entire heap
        let mut ptr: *mut u8 = self.memory;
        while ptr < self.memory.add(self.size) {
            // every time we reach the beginning of this loop, ptr should point to a Data struct.
            let data = *(ptr as *const Data);
            let align_offset = ptr.add(size_of::<Data>()).align_offset(layout.align());
            let actual_size = layout.size() + align_offset;

            // if this block is used, go to the next one.
            if data.used {
                if ptr.add(data.size).add(size_of::<Data>()) > self.memory.add(self.size) {
                    panic!("Ran out of memory while allocating {} bytes", layout.size());
                }
                ptr = ptr.add(data.size + size_of::<Data>());
                continue;
            }

            // if we've reached the end of allocated memory (which is contiguous)
            if data.size == 0 {
                self.panic_if_oom(ptr, actual_size);

                crate::interrupts::enable();
                return self.alloc_raw(ptr, actual_size, true).add(align_offset);
            }

            // if our data fits in this block
            if data.size >= actual_size {
                crate::interrupts::enable();
                return self.alloc_chunk(ptr, actual_size, data.size).add(align_offset);
            }

            // otherwise, it's possible that we have a couple of unused blocks in a row which can be
            // combined to fit our data.
            let mut size_sum: usize = data.size;
            let root_ptr = ptr;
            let mut success = true;
            let mut com_data = *(ptr as *const Data);
            while size_sum < actual_size {
                ptr = ptr.add(com_data.size + size_of::<Data>()); // go to the next block
                com_data = *(ptr as *const Data);
                if com_data.used { 
                    success = false;
                    break;
                }
                if com_data.size == 0 {
                    size_sum += self.panic_if_oom(ptr, actual_size);
                    success = size_sum >= actual_size;
                    if success {
                        self.alloc_raw(root_ptr.add(size_sum), 0, false);
                    }
                    break;
                }
                size_sum += com_data.size;
            }

            if success {
                return self.alloc_chunk(root_ptr, actual_size, size_sum).add(align_offset);
            }
        }

        panic!(
            "memory allocation of {} bytes failed REALLY badly. if you see this message \
            i think it's an OOM but you really NEVER should see this no matter what",
            layout.size()
        );
    }

    pub(crate) unsafe fn dealloc_internal(&self, ptr: *mut u8, _: Layout) {
        // unset the used flag for the memory block that starts at ptr
        (*(ptr.sub(size_of::<Data>()) as *mut Data)).used = false;
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut out = core::ptr::null_mut();
        crate::syscall::Alloc::call(&mut out, layout);
        out
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        crate::syscall::Dealloc::call(ptr, layout);
    }
}

#[global_allocator]
pub(crate) static mut HEAP: Heap = Heap { memory: 0 as *mut u8, size: 0 }; // gets initialized at init()

pub(crate) unsafe fn init(space_start: usize, size: usize) {
    let actual_size = size - (space_start - 0x100_000);
    HEAP = Heap { memory: space_start as *mut u8, size: actual_size };

    (*paging::PageDirectory::curr()).map_addresses(
        space_start,
        space_start + actual_size,
        space_start,
        PageFlags::RW | PageFlags::USER
    );

    // in order to ensure that the USED flag (in Data) is false (0) for unallocated memory,
    // we zero out our entire heap.
    for i in 0..actual_size/4 {
        *((space_start+4*i) as *mut u32) = 0;
        if i % 500_000 == 0 {
            crate::println!("{:.2}% done clearing heap..", (i as f32)/(size as f32/4.0)*100.0);
        }
    }

    HAS_INIT = true;
}

// No need for mutex as we'll be modifying it exactly once, after init()
static mut HAS_INIT: bool = false;
pub(crate) fn has_init() -> bool { unsafe { HAS_INIT } }
