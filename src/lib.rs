#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

use crate::io::Clear;
use crate::vga_console::CONSOLE;
use core::arch::asm;
use core::panic::PanicInfo;

pub mod interrupts;
pub mod io;
pub mod keyboard;
pub mod pic;
pub mod vga_console;
pub mod timer;
pub mod heap;
mod grub;
pub mod events;
pub mod console;
pub mod ata;
pub mod fs;

#[no_mangle]
pub(crate) extern "C" fn main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();
    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're dividing by 50 (not using our entire available memory) to get faster loading times.
        heap::init(0x100_000, info.mem_upper * 1024 / 50);
    }

    CONSOLE.lock().clear();
    pic::remap();
    timer::init();
    keyboard::init();
    interrupts::init();
    console::init();
    ata::init();

    fs::File::create("/fuck/you/lol.txt");

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
