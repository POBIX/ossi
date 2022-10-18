#![no_std]
#![no_main]

use core::panic::PanicInfo;

pub mod io;
pub mod vga_console;

#[no_mangle]
pub extern "C" fn main() -> ! {
    println!("Hello world! {} {} {}", 1, 1.558, "jhgf");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
