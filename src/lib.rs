#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![feature(pointer_byte_offsets)]
#![feature(int_roundings)]
#![feature(const_mut_refs)]
#![no_std]
#![no_main]

extern crate alloc;

use alloc::alloc::{alloc, dealloc};
use io::Write;
use paging::PageFlags;

use crate::io::{Clear, Read};
use crate::vga_console::CONSOLE;
use core::alloc::Layout;
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

    let heap_start_addr = paging::init();

    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're using a maximum of a 50MB to get faster loading times,
        // and only start at heap_start_addr since some of the heap was used by paging.
        heap::init(heap_start_addr, core::cmp::min(50 * 1024 * 1024, info.mem_upper));
    }

    CONSOLE.lock().clear();
    pic::remap();
    timer::init();
    keyboard::init();
    interrupts::init();
    console::init();
    ata::init();

    let mut file = fs::File::open("/cool.exe").unwrap();
    // The array is opcodes for [mov ebx, 5; mov eax, ebx; ret]
    file.write_bytes(&[ 0xBBu8, 0x05, 0x00, 0x00, 0x00, 0x89, 0xD8, 0xC3 ]);

    unsafe {
        let l = execution::run_program(0, &file.read_bytes(8));
        println!("{}", l);
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
