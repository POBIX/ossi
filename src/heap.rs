//! TODO: Alignment
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

use alloc::alloc::{GlobalAlloc, Layout};
use spin::{Mutex, Once};

use crate::{
    interrupts::KernelInterruptGuard,
    paging::{PAGE_SIZE, self, PageFlags},
};

pub trait Pager {
    unsafe fn map(&mut self, n: usize) -> usize;
    unsafe fn unmap(&mut self, page: usize);
}

pub struct Heap<P : Pager> {
    pager: P,
    first_slab: *mut SlabMetadata,
    last_slab: *mut SlabMetadata,
}

pub struct SlabNode {
    next: *mut SlabNode,
}

pub struct SlabMetadata {
    block_size: u16,
    addr: *mut u8,
    free: *mut SlabNode,
    next: *mut SlabMetadata,
}

// The choice of numbers is pretty much completely arbitrary. I stole these numbers from Linux.
const SLAB_SIZES: &[u16] = &[8, 16, 32, 64, 96, 128, 192, 256, 512, 1024, 2048];
fn get_slab_size(req: usize) -> Option<u16> {
    SLAB_SIZES.iter().find(|&&n| usize::from(n) >= req).copied()
}

impl<P: Pager> Heap<P> {
    pub fn new(pager: P) -> Heap<P> { Heap {
        pager,
        first_slab: core::ptr::null_mut(),
        last_slab: core::ptr::null_mut()
    }}

    fn create_metadata(&mut self, block_size: u16, node: *mut SlabNode) -> *mut SlabMetadata {
        const S: usize = core::mem::size_of::<SlabMetadata>();
        // The metadatas are packed sequentially into entire pages.
        // So, if we haven't exhausted the current page we can just go to the next address
        let new: *mut SlabMetadata =
            if !self.first_slab.is_null() && self.last_slab as usize % PAGE_SIZE < PAGE_SIZE - S {
                self.last_slab.wrapping_add(1)
            } else {
                // If we have then we make a new page and put this metadata at its top
                unsafe { self.pager.map(1) as *mut SlabMetadata }
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
            unsafe {
                (*self.last_slab).next = new;
            }
            self.last_slab = new;
        }

        self.last_slab
    }

    fn create_slab(&mut self, block_size: u16) -> *mut SlabMetadata {
        // Make new page that our slab will live in
        let mut node = unsafe { self.pager.map(1) } as *mut SlabNode;
        let metadata: *mut SlabMetadata = self.create_metadata(block_size, node);

        // Make free slots (all of them since we just created the slab) point at the next free slot
        for _ in 0..(PAGE_SIZE / block_size as usize - 1) {
            unsafe {
                (*node).next = (node as usize + block_size as usize) as *mut SlabNode;
                node = (*node).next;
            }
        }

        unsafe {
            (*node).next = core::ptr::null_mut();
        } // No next free for the last one!

        metadata
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
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
                self.pager.map(layout.size().div_ceil(PAGE_SIZE)) as *mut u8
            }
        }
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        if layout.size() > *SLAB_SIZES.last().unwrap() as usize {
            // This wasn't a slab allocation (too big), we just need to unmap its pages
            for i in 0..layout.size().div_ceil(PAGE_SIZE) {
                self.pager.unmap(ptr as usize / PAGE_SIZE + i);
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

struct KernelPager {
    start: usize,
    end: usize
}

impl KernelPager {
    // In page table space
    fn new(start: usize, size: usize) -> KernelPager { KernelPager {
        start,
        end: start + size
    }}
}

impl Pager for KernelPager {
    unsafe fn map(&mut self, n: usize) -> usize {
        paging::get_kernel_directory_mut().map_new_pages(n, self.start, self.end, PageFlags::RW)
    }

    unsafe fn unmap(&mut self, page: usize) {
        paging::get_kernel_directory_mut().unmap(page, true)
    }
}

struct KernelHeap(Once<Mutex<Heap<KernelPager>>>);
unsafe impl Sync for KernelHeap {} // Interrupts must always be disabled before modifying the heap
unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = KernelInterruptGuard::new();
        self.0.get().unwrap().lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _guard = KernelInterruptGuard::new();
        self.0.get().unwrap().lock().dealloc(ptr, layout)
    }
}

#[global_allocator]
static KERNEL_HEAP: KernelHeap = KernelHeap(Once::new());

pub(crate) fn init_kernel(space_start: usize, size: usize) {
    KERNEL_HEAP.0.call_once(|| Mutex::new(Heap::new(KernelPager::new(space_start, size))));
}

pub(crate) fn has_init_kernel() -> bool { KERNEL_HEAP.0.is_completed() }
