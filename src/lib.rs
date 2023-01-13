#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![feature(pointer_byte_offsets)]
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
pub mod execution;
pub mod paging;

#[no_mangle]
pub(crate) extern "C" fn main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();
    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're using a maximum of a 50MB to get faster loading times.
        heap::init(0x100_000, core::cmp::min(50 * 1024 * 1024, info.mem_upper));
    }

    CONSOLE.lock().clear();
    pic::remap();
    timer::init();
    keyboard::init();
    interrupts::init();
    console::init();
    ata::init();

    unsafe {
        use paging::*;
        let directory = create_page_directory();
        let table = create_page_table();
        directory[0] = addr_flags(table.as_ptr() as u32, PageDirectoryFlags::READ_WRITE | PageDirectoryFlags::PRESENT);
        paging::enable(directory.as_ptr());
    }

    println!("Just making sure everything still works :)");

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
