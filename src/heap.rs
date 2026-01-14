use alloc::alloc::{GlobalAlloc, Layout};
use crate::paging::{*};

#[derive(Clone, Copy)]
pub(crate) struct ProcessHeapData {
    counter: usize
}
impl ProcessHeapData {
    pub(crate) fn new() -> Self {
        Self { counter: 0 }
    }
}

pub struct Heap;

impl Heap {
    pub(crate) unsafe fn alloc_internal(&self, layout: Layout) -> *mut u8 {
        let whole_pages: usize = layout.size() / PAGE_SIZE;
        let part_page: usize = layout.size() % PAGE_SIZE;

        let proc: crate::process::Process = crate::process::get_curr_process();
        let dir: &mut crate::paging::PageDirectory = proc.ctx.as_ref().dir.as_mut().unwrap(); // can't be null if the process exists

        let usage = FRAMES_USAGE.lock();
        
        for i in 0..whole_pages {
            let free: usize = usage.get_free_frame();
            let page: &Page = dir.make_page(proc.data.counter, PageFlags::RW | PageFlags::USER, false).unwrap();
        }

        core::ptr::null_mut()
    }

    pub(crate) unsafe fn dealloc_internal(&self, layout: Layout) {

    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut out = core::ptr::null_mut();
        crate::syscall::Alloc::call(&mut out, layout);
        out
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        crate::syscall::Dealloc::call(ptr, layout)
    }
}


pub(crate) unsafe fn init(space_start: usize, size: usize) {
    
}
