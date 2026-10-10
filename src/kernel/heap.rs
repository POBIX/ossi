    use crate::heap::*;
    use crate::paging::PageFlags;
    use crate::kernel::paging;
    use alloc::alloc::{GlobalAlloc, Layout};
    use spin::{Mutex, Once};
    use crate::kernel::interrupts::KernelInterruptGuard;

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
    static HEAP: KernelHeap = KernelHeap(Once::new());

    pub(crate) fn init(space_start: usize, size: usize) {
        HEAP.0.call_once(|| Mutex::new(Heap::new(KernelPager::new(space_start, size))));
    }

    pub(crate) fn has_init() -> bool { HEAP.0.is_completed() }
