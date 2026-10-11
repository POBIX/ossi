use core::arch::asm;

use crate::io::Write;
use crate::kernel::interrupts;
use crate::interrupts::GateType;
use crate::paging::{PageFlags, PageDirectory};
use crate::kernel::vga_console::CONSOLE;

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(
            syscall_handler as *const () as u32, GateType::DInterrupt, 3
        );
    }
}

unsafe extern "x86-interrupt" fn syscall_handler() {
    asm!(
        "cli",
        "push ebx", // syscall type
        "push eax", // syscall data
        "call syscall_handler_inner",
        "add esp, 8", // pop the parameters
        // We don't need to restore interrupts, `iret` does so for us (inserted at the end of the
        // function by "x86-interrupt"), provided the syscall originated from ring 3. Therefore the
        // kernel itself must never place a syscall.
    );
}

pub(crate) fn print(str: *const u8, len: usize) {
    unsafe {
        CONSOLE.lock().write_string(core::str::from_utf8(core::slice::from_raw_parts(str, len)).unwrap());
    }
}

pub(crate) fn halt() { unsafe { asm!("hlt"); } }

pub(crate) fn empty() {}

pub(crate) unsafe fn map_pages(n: usize, start: usize, end: usize, flags: PageFlags) -> usize {
    (*PageDirectory::curr()).map_new_pages(n, start, end, flags)
}

pub(crate) unsafe fn unmap_page(page: usize) {
    (*PageDirectory::curr()).unmap(page, true);
}
