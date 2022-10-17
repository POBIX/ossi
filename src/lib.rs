#![no_std]
#![no_main]

use core::panic::PanicInfo;
use crate::vga_console::Console;
use crate::io::{Write};

pub mod io;
pub mod vga_console;

#[no_mangle]
pub extern "C" fn main() -> ! {
    let mut console = Console::new();
    console.write_bytes(b"Hello, world!");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
