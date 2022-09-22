#![no_std]
#![no_main]

mod vga_console;
mod io;

use core::panic::PanicInfo;
use crate::io::{Clear, Write};
use crate::vga_console::{Color, ColorCode, Console};

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
