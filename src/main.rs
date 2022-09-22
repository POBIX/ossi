#![no_std]
#![no_main]

pub mod vga_console;
pub mod io;

use core::panic::PanicInfo;
use crate::io::{Clear, Write};
use crate::vga_console::{Console};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut console = Console::new();
    console.clear();
    console.write_bytes(b"Hello, world!");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
