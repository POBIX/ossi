#![no_std]
#![no_main]

// pub mod vga_console;
// pub mod io;

use core::panic::PanicInfo;
// use crate::io::{Clear, Write};
// use crate::vga_console::{Console};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // let mut console = Console::new();
    // console.write_byte(b'a');
    unsafe {
        let vga = 0xB8000 as *mut u8;
        *vga = b'a';
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
