#![feature(abi_x86_interrupt)]
#![feature(pointer_byte_offsets)]
#![feature(int_roundings)]
#![feature(const_mut_refs)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(new_uninit)]
#![feature(inline_const)]
#![no_std]
#![no_main]

extern crate alloc;


use core::panic::PanicInfo;

pub mod interrupts;
pub mod io;
pub mod keyboard;
pub mod pic;
pub mod vga_console;
pub mod heap;
pub mod events;
pub mod fs;
pub mod paging;
pub mod syscall;
pub mod process;

#[cfg(feature = "kernel")]
mod kernel;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
