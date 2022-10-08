#![no_std]
#![no_main]

use core::panic::PanicInfo;
use crate::vga_console::Console;
use crate::io::{Write};

pub mod io;
pub mod vga_console;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        *(0xb8000 as *mut u8) = b'a';
        *(0xb8001 as *mut u8) = b'a';
        *(0xb8002 as *mut u8) = b'a';
        *(0xb8003 as *mut u8) = b'a';
        *(0xb8004 as *mut u8) = b'a';
        *(0xb8005 as *mut u8) = b'a';
        *(0xb8006 as *mut u8) = b'a';
        *(0xb8007 as *mut u8) = b'a';
        *(0xb8008 as *mut u8) = b'a';
        *(0xb8009 as *mut u8) = b'a';
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
