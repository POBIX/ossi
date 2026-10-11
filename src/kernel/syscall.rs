use crate::paging::{PageFlags, PageDirectory};
use crate::syscall::*;
use core::arch::asm;
use spin::{Mutex, Lazy};
use crate::events::Event;
use crate::interrupts::GateType;
use crate::kernel::interrupts;

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(
            syscall_handler as *const () as u32, GateType::DInterrupt, 3
        );
        interrupts::IDT[0x81] = interrupts::Handler::new_raw(
            syscall_handler_lifetime as *const () as u32, GateType::DInterrupt, 3
        );
    }
}

extern "x86-interrupt" fn syscall_handler() {
    unsafe {
        asm!(
            "cli",
            "push ebx",
            "push eax",
            "call syscall_handler_inner",
            "add esp, 8", // pop the parameters
            "sti"
        )
    }
}

extern "x86-interrupt" fn syscall_handler_lifetime() {
    unsafe {
        asm!(
            "cli",
            "push ebx",
            "push eax",
            "call syscall_handler_inner_lifetime",
            "add esp, 8", // pop the parameters
            "sti"
        )
    }
}

pub(crate) fn print_syscall(args: core::fmt::Arguments) {
    crate::kernel::vga_console::_print(args);
}

pub(crate) fn is_key_pressed(out: &mut bool, key: crate::keyboard::Key) {
    *out = crate::kernel::keyboard::is_key_pressed(key);
}

pub(crate) fn empty() {}

pub(crate) fn halt() { unsafe { asm!("hlt"); } }

pub(crate) unsafe fn set_isr(index: usize, func: extern "x86-interrupt" fn(), dpl: u8) {
    interrupts::IDT[index] = interrupts::Handler::new(func, GateType::DInterrupt, dpl);
}

pub(crate) unsafe fn map_pages(n: usize, from: usize, to: usize, flags: PageFlags, out: *mut usize) {
    *out = (*PageDirectory::curr()).map_new_pages(n, from, to, flags);
}

pub(crate) unsafe fn unmap_pages(page: usize) {
    (*PageDirectory::curr()).unmap(page, true)
}

macro_rules! generate_ret_func {
    ($name: ident, $expr: expr, $type: ty) => {
        pub(crate) fn $name(out: &mut $type) {
            *out = $expr;
        }
    };
}

generate_ret_func!(has_loaded_processes, crate::kernel::process::has_loaded_processes(), bool);
generate_ret_func!(get_curr_page_dir, crate::paging::PageDirectory::curr(), *mut crate::paging::PageDirectory);
generate_ret_func!(are_interrupts_enabled, crate::kernel::interrupts::is_enabled(), bool);
generate_ret_func!(get_on_key_down, &crate::kernel::keyboard::ON_KEY_DOWN, &'static Mutex<Event<crate::keyboard::KeyArgs>>);
generate_ret_func!(get_on_key_up, &crate::kernel::keyboard::ON_KEY_UP, &'static Mutex<Event<crate::keyboard::KeyArgs>>);
generate_ret_func!(get_console, &crate::kernel::vga_console::CONSOLE, &'static Lazy<Mutex<crate::vga_console::Console>>);
generate_ret_func!(is_caps_lock_active, crate::kernel::keyboard::is_caps_lock_active(), bool);
generate_ret_func!(fs_get_header, &crate::kernel::fs::HEADER, &'static Lazy<Mutex<&'static mut crate::fs::Header>>);
