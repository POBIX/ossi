use core::mem::size_of;

use alloc::alloc::{GlobalAlloc, Layout};

//TODO: paging, permissions

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

struct Heap {
    memory: *mut u8,
    size: usize,
}

impl Heap {
    /// allocates memory without doing any checks whatsoever.
    #[inline]
    unsafe fn alloc_raw(&self, ptr: *mut u8, size: usize) -> *mut u8 {
        *(ptr as *mut Data) = Data { used: true, size };
        ptr.add(size_of::<Data>()) // address of actual user data (without the struct)
    }

    /// reallocates previously deallocated memory and creates a new unused block in the remaining space
    unsafe fn alloc_chunk(&self, ptr: *mut u8, size: usize) -> *mut u8 {
        let mut data = *(ptr as *mut Data);

        let remaining_space = size - data.size;

        // if there isn't enough space left to create another block, simply mark the existing one as used
        // and return the pointer, with a little bit of leftover space.
        if remaining_space <= size_of::<Data>() {
            data.used = true;
            return ptr.add(size_of::<Data>());
        }

        self.alloc_raw(ptr.add(remaining_space).add(size_of::<Data>()), remaining_space);
        self.alloc_raw(ptr, size)
    }

    /// this function should be called to check whether there is enough available memory
    /// if data.size==0 (the last allocated block of memory).
    /// panics if out of memory, returns the amount of available memory otherwise.
    #[inline]
    unsafe fn panic_if_oom(&self, ptr: *const u8, size: usize) -> usize {
        // rem = self.memory + self.size - (ptr + size)
        let rem = (self.memory.add(self.size) as isize) - (ptr.add(size) as isize);
        if rem < 0 {
            panic!("OOM: tried to allocate {} bytes, ran out of memory!", size)
        }
        rem as usize
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        crate::interrupts::disable(); // prevent two simultaneous allocations (TODO: until threads)

        // iterate over our entire heap
        let mut ptr: *mut u8 = self.memory;
        while ptr < self.memory.add(self.size) {
            // every time we reach the beginning of this loop, ptr should point to a Data struct.
            let data = *(ptr as *const Data);
            let align_offset = ptr.add(size_of::<Data>()).align_offset(layout.align());
            let actual_size = layout.size() + align_offset;

            // if this block is used, go to the next one.
            if data.used {
                ptr = ptr.add(data.size + size_of::<Data>());
                continue;
            }

            // if we've reached the end of allocated memory (which is contiguous)
            if data.size == 0 {
                self.panic_if_oom(ptr, actual_size);

                crate::interrupts::enable();
                return self.alloc_raw(ptr, actual_size).add(align_offset);
            }

            // if our data fits in this block
            if data.size >= actual_size {
                crate::interrupts::enable();
                return self.alloc_chunk(ptr, actual_size).add(align_offset);
            }

            // otherwise, it's possible that we have a couple of unused blocks in a row which can be
            // combined to fit our data.
            let mut size_sum: usize = data.size;
            let root_ptr = ptr;
            let mut success = true;
            while size_sum < actual_size {
                ptr = ptr.add(data.size + size_of::<Data>()); // go to the next block
                let data = *(ptr as *const Data);
                if data.used { 
                    success = false;
                    break;
                }
                if data.size == 0 {
                    size_sum += self.panic_if_oom(ptr, actual_size);
                    success = size_sum >= actual_size;
                    break;
                }
                size_sum += data.size;
            }

            if success {
                return self.alloc_raw(root_ptr, size_sum).add(actual_size);
            }
        }

        panic!(
            "memory allocation of {} bytes failed REALLY badly. if you see this message \
            i think it's an OOM but you really NEVER should see this no matter what",
            layout.size()
        );
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        // unset the used flag for the memory block that starts at ptr
        (*(ptr.sub(size_of::<Data>()) as *mut Data)).used = false;
    }
}

#[global_allocator]
static mut HEAP: Heap = Heap { memory: 0 as *mut u8, size: 0 }; // gets initialized at init()

pub(crate) unsafe fn init(space_start: usize, size: usize) {
    HEAP = Heap { memory: space_start as *mut u8, size };

    // in order to ensure that the USED flag (in Data) is false (0) for unallocated memory,
    // we zero out our entire heap.
    // TODO: don't do this. it's too slow.
    for i in 0..size/4 {
        *((space_start+4*i) as *mut u32) = 0;
        if i % 500_000 == 0 {
            crate::println!("{:.2}% done clearing heap..", (i as f32)/(size as f32/4.0)*100.0);
        }
    }
}