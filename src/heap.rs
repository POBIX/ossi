//! Brief overview of how allocation works:
//! The page directory keeps track of which pages are mapped and which are not.
//! For a mapped page, there are two possibilites:
//! 1. It is part of an allocation > 4KB. So long as it is mapped the *entire* page belongs to that
//!    allocation only. There is no metadata.
//! 2. It contains many allocations, all of the same size. For example it might contain 128 32-byte
//!    blocks. Whenever a 32-byte allocation is made, we find a free slot within a 32-byte
//!    designated page (Slab), and mark it as used. 
//! In order to find free slots efficiently, the blocks are structured as a linked list of free
//! slots: a used slot contains arbitrary data; a free slot is a SlabNode.
//! The metadata of the slabs is stored in a linked list as well.

use core::sync::atomic::{AtomicBool, Ordering};

use alloc::alloc::{GlobalAlloc, Layout};
use spin::Mutex;

use crate::{paging::{PageDirectory, PAGE_SIZE, PageFlags}, interrupts::KernelInterruptGuard};

pub(crate) struct Heap {
    space_start: usize,
    space_end: usize,
    first_slab: *mut SlabMetadata,
    last_slab: *mut SlabMetadata
}

pub(crate) struct SlabNode {
    next: *mut SlabNode,
}

pub(crate) struct SlabMetadata {
    block_size: u16,
    addr: *mut u8,
    free: *mut SlabNode,
    next: *mut SlabMetadata,
}

// The choice of numbers is pretty much completely arbitrary. I stole these numbers from Linux.
const SLAB_SIZES: &[u16] = &[8, 16, 32, 64, 96, 128, 192, 256, 512, 1024, 2048];
fn get_slab_size(req: usize) -> Option<u16> { SLAB_SIZES.iter().find(|&&n| usize::from(n) >= req).copied() }

impl Heap {
    fn create_metadata(&mut self, block_size: u16, node: *mut SlabNode) -> *mut SlabMetadata {
        const S: usize = core::mem::size_of::<SlabMetadata>();
        // The metadatas are packed sequentially into entire pages.
        // So, if we haven't exhausted the current page we can just go to the next address
        let new: *mut SlabMetadata = if !self.first_slab.is_null() && self.last_slab as usize % PAGE_SIZE < PAGE_SIZE - S {
            self.last_slab.wrapping_add(1)
        } else {
            // If we have then we make a new page and put this metadata at its top
            let dir: &mut PageDirectory = unsafe { PageDirectory::curr().as_mut().unwrap() };
            dir.map_new_page(self.space_start, self.space_end, PageFlags::USER | PageFlags::RW)
                as *mut SlabMetadata
        };

        unsafe {
            (*new).block_size = block_size;
            (*new).addr = node as *mut u8;
            (*new).free = node;
            (*new).next = core::ptr::null_mut();
        }

        if self.first_slab.is_null() {
            self.first_slab = new;
            self.last_slab = new; 
        } else {
            unsafe { (*self.last_slab).next = new; }
            self.last_slab = new;
        }
        
        self.last_slab
    }

    fn create_slab(&mut self, block_size: u16) -> *mut SlabMetadata {
        // Make new page that our slab will live in
        let dir: &mut PageDirectory = unsafe { PageDirectory::curr().as_mut().unwrap() };
        let mut node = dir.map_new_page(self.space_start, self.space_end, PageFlags::USER | PageFlags::RW) as *mut SlabNode;
        let metadata: *mut SlabMetadata = self.create_metadata(block_size, node);

        // Make free slots (all of them since we just created the slab) point at the next free slot
        for _ in 0..(PAGE_SIZE / block_size as usize - 1) {
            unsafe {
                (*node).next = (node as usize + block_size as usize) as *mut SlabNode;
                node = (*node).next;
            }
        }
        unsafe { (*node).next = core::ptr::null_mut(); } // No next free for the last one!

        metadata
    }

    pub(crate) unsafe fn alloc_internal(&mut self, layout: Layout) -> *mut u8 {
        let _guard = KernelInterruptGuard::new();

        match get_slab_size(layout.size()) {
            Some(size) => {
                // Find non-exhausted slab with appropriate size, or create one
                let mut slab: *mut SlabMetadata = self.first_slab;
                while !slab.is_null() && ((*slab).block_size != size || (*slab).free.is_null()) {
                    slab = (*slab).next;
                }
                if slab.is_null() {
                    slab = self.create_slab(size);
                }

                // Take our free block, advance the pointer
                let out: *mut SlabNode = (*slab).free;
                (*slab).free = (*out).next;

                out as *mut u8
            }
            None => {
                // This is a large allocation, the slab allocator won't help us.
                // Just give them a few pages
                let dir: &mut PageDirectory = PageDirectory::curr().as_mut().unwrap();
                dir.map_new_pages(
                    layout.size().div_ceil(PAGE_SIZE), self.space_start, self.space_end, 
                    PageFlags::USER | PageFlags::RW
                ) as *mut u8
            }
        }
    }

    pub(crate) unsafe fn dealloc_internal(&mut self, ptr: *mut u8, layout: Layout) {
        let _guard = KernelInterruptGuard::new();

        if layout.size() > *SLAB_SIZES.last().unwrap() as usize {
            // This wasn't a slab allocation (too big), we just need to unmap its pages
            let dir: &mut PageDirectory = PageDirectory::curr().as_mut().unwrap();
            for i in 0..(layout.size().div_ceil(PAGE_SIZE)) {
                dir.unmap(ptr as usize / PAGE_SIZE + i);
            }
        } else {
            // Go through each slab and see if our pointer is in it
            let mut slab: *mut SlabMetadata = self.first_slab;
            while !slab.is_null() && (
                (ptr as usize) < ((*slab).addr as usize) || 
                (ptr as usize) >= ((*slab).addr as usize) + PAGE_SIZE
            ) {
                slab = (*slab).next;
            }
            if slab.is_null() {
                panic!("Attempted to free unmapped pointer {:?}!", ptr);
            }

            // Reinterpret whatever data was there as a SlabNode and make it top of the free list
            let node = ptr as *mut SlabNode;
            (*node).next = (*slab).free;
            (*slab).free = node;
        }
    }
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

pub(crate) struct GlobalHeap(pub Mutex<Heap>);
unsafe impl Sync for GlobalHeap {} // Interrupts must always be disabled before modifying the heap

#[global_allocator]
pub(crate) static HEAP: GlobalHeap = GlobalHeap(Mutex::new(Heap { 
    space_start: 0, space_end: 0, 
    first_slab: core::ptr::null_mut(), last_slab: core::ptr::null_mut() 
}));

pub(crate) unsafe fn init(space_start: usize, size: usize) {
    let mut heap = HEAP.0.lock();
    heap.space_start = space_start;
    heap.space_end = space_start + size;
    HAS_INIT.store(true, Ordering::Relaxed);
}

static HAS_INIT: AtomicBool = AtomicBool::new(false);
pub(crate) fn has_init() -> bool { HAS_INIT.load(Ordering::Relaxed) }
