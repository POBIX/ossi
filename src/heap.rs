use alloc::alloc::{GlobalAlloc, Layout};

struct Heap {

}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        todo!()
    }
}

#[global_allocator]
static HEAP: Heap = Heap {};
