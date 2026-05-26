//! Brief overview of how allocation works:
//! The page directory keeps track of which pages are mapped and which are not.
//! For a mapped page, there are two possibilites:
//! 1. It is part of an allocation > 4KB. So long as it is mapped the *entire* page belongs to that
//!    allocation only. There is no metadata.
//! 2. It contains many allocations, all of the same size. For example it might contain 128 32-byte
//!    blocks. Whenever a 32-byte allocation is made, we find a free slot within a 32-byte
//!    designated page (Slab), and mark it as used.
//!    In order to find free slots efficiently, the blocks are structured as a linked list of free
//!    slots. The metadata of the slabs is stored in a linked list as well.

use alloc::alloc::{GlobalAlloc, Layout};
use spin::Once;

use crate::paging::{PageDirectory, PAGE_SIZE, FRAMES_USAGE, PageFlags};

struct Heap {
    space_start: usize,
    space_end: usize,
    slabs: *mut SlabMetadata
}

struct SlabNode {
    next: *mut SlabNode
}

struct SlabMetadata {
    block_size: u16,
    first: *mut SlabNode,
    free: *mut SlabNode,
    next: *mut SlabMetadata
}

impl Heap {
    fn create_slab(&self, block_size: u16) -> *mut SlabMetadata {
        // Find a free page and frame (virtual and physical addresses) to put the slab in
        let dir: &mut PageDirectory = unsafe { PageDirectory::curr().as_mut().unwrap() };
        let page: usize = dir.get_free_page(self.space_start, self.space_end).expect("Out of virtual memory!");
        let frame: usize = FRAMES_USAGE.lock().get_free_frame().expect("Out of memory!") * PAGE_SIZE;
        // Map them and interpret as slab
        let node: *mut SlabNode = unsafe {
            dir.make_page(page, frame, PageFlags::USER | PageFlags::RW).unwrap();
            page as *mut SlabNode
        };

        // Make free slots (all of them since we just created the slab) point at the next free slot
        for i in 0..(PAGE_SIZE / block_size - 1) {
            unsafe {
                (*node).next = ((node as usize) + i * block_size) as *mut SlabNode;
                node = (*node).next;
            }
        }
        unsafe { (*node).next = 0; }

        slab
    }
}

impl Heap {
    pub(crate) unsafe fn alloc_internal(&self, layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    pub(crate) unsafe fn dealloc_internal(&self, ptr: *mut u8, _: Layout) { }
}

unsafe impl GlobalAlloc for GlobalHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut out = core::ptr::null_mut();
        crate::syscall::Alloc::call(&mut out, layout);
        out
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        crate::syscall::Dealloc::call(ptr, layout);
    }
}

struct GlobalHeap(pub Once<Heap>);

#[global_allocator]
pub(crate) static HEAP: GlobalHeap = GlobalHeap(Once::new());

pub(crate) unsafe fn init(space_start: usize, size: usize) {
    HAS_INIT = true;
}

// No need for mutex as we'll be modifying it exactly once, after init()
static mut HAS_INIT: bool = false;
pub(crate) fn has_init() -> bool { unsafe { HAS_INIT } }
