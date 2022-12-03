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

#[no_mangle]
pub(crate) extern "C" fn main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();
    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're dividing by 50 (not using our entire available memory) to get faster loading times.
        heap::init(0xA00_000, info.mem_upper * 1024 / 50);
    }

    CONSOLE.lock().clear();
    pic::remap();
    timer::init();
    keyboard::init();
    interrupts::init();
    console::init();
    ata::init();

    let mut buffer: [u8; 512] = [0; 512];
    unsafe {
        ata::read_sectors(0, &mut buffer);
    }
    for i in 0..buffer.len() {
        print!("{:0X}", buffer[i]);
        buffer[i] = i as u8;
    }
    unsafe {
        ata::write_sectors(0, &buffer);
    }

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
