use core::arch::asm;

use crate::io::{Write, Read, Seek, Clear};
use crate::kernel::interrupts;
use crate::interrupts::GateType;
use crate::paging::{self, PageFlags, PageDirectory};
use crate::kernel::vga_console::CONSOLE;
use crate::vga_console::ColorCode;

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

pub(crate) unsafe fn map_pages(n: usize, flags: PageFlags) -> usize {
    (*PageDirectory::curr()).map_new_pages(n, paging::USER_MEM_FROM, paging::USER_MEM_TO, flags)
}

pub(crate) unsafe fn unmap_page(page: usize) {
    (*PageDirectory::curr()).unmap(page, true);
}

macro_rules! console_syscalls {
    ( $( $method:ident { $($arg:ident : $ty:ty),* } $(-> $ret:ty)?),* ) => {
        $(
            pub(crate) fn $method($($arg: $ty),*) $(-> $ret)? {
                CONSOLE.lock().$method($($arg),*)
            }
        )*
    };
}

console_syscalls!(
    write_byte{byte: u8},
    read_byte{} -> u8,
    seek{pos: usize},
    get_cursor_position{} -> usize,
    seek_raw{pos: usize},
    update_cursor{},
    newline{},
    newline_raw{},
    backspace{},
    clear{},
    set_color{color: ColorCode},
    get_color{} -> ColorCode
);

pub(crate) unsafe fn write_string(ptr: *const u8, len: usize) {
    let str = core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).unwrap();
    CONSOLE.lock().write_string(str)
}
