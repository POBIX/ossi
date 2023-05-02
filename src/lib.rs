#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![feature(pointer_byte_offsets)]
#![feature(int_roundings)]
#![feature(const_mut_refs)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(new_uninit)]
#![no_std]
#![no_main]

extern crate alloc;

use crate::io::Clear;
use crate::vga_console::CONSOLE;
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
mod userspace;
pub mod syscall;

extern "C" {
    static CODE_SEG: usize;
    static DATA_SEG: usize;
    static KERNEL_STACK_TOP: usize;
}

#[no_mangle]
pub(crate) extern "C" fn main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();

    pic::remap();
    timer::init();
    interrupts::init();

    let heap_start_addr = paging::init();

    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're using a maximum of 5MB to get faster loading times,
        // and only start at heap_start_addr since some of the heap was used by paging.
        heap::init(heap_start_addr, core::cmp::min(5 * 1024 * 1024, info.mem_upper * 1024));
    }

    userspace::init();
    syscall::init();

    CONSOLE.lock().clear();
    keyboard::init();
    console::init();
    ata::init();

    unsafe {
        println!("{:p} {:p}", &CODE_SEG, &DATA_SEG);
    }

    unsafe {
        execution::run_program(&[0x7F , 0x45 , 0x4C , 0x46 , 0x01 , 0x01 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x02 , 0x00 , 0x03 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x80 , 0x80 , 0x04 , 0x08 , 0x34 , 0x00 , 0x00 , 0x00 , 0x98 , 0x02 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x34 , 0x00 , 0x20 , 0x00 , 0x02 , 0x00 , 0x28 , 0x00 , 0x07 , 0x00 , 0x06 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x80 , 0x04 , 0x08 , 0x00 , 0x80 , 0x04 , 0x08 , 0x07 , 0x01 , 0x00 , 0x00 , 0x07 , 0x01 , 0x00 , 0x00 , 0x05 , 0x00 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x00 , 0x51 , 0xE5 , 0x74 , 0x64 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x06 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x83 , 0xEC , 0x08 , 0x8D , 0x05 , 0xD6 , 0x80 , 0x04 , 0x08 , 0x89 , 0x04 , 0x24 , 0xC7 , 0x44 , 0x24 , 0x04 , 0x0F , 0x00 , 0x00 , 0x00 , 0xE8 , 0x17 , 0x00 , 0x00 , 0x00 , 0x83 , 0xC4 , 0x08 , 0xC3 , 0x66 , 0x90 , 0x90 , 0x8B , 0x44 , 0x24 , 0x04 , 0xEB , 0xFE , 0x66 , 0x90 , 0x66 , 0x90 , 0x66 , 0x90 , 0x66 , 0x90 , 0x66 , 0x90 , 0x53 , 0x83 , 0xEC , 0x10 , 0x8B , 0x44 , 0x24 , 0x1C , 0x8B , 0x4C , 0x24 , 0x18 , 0x89 , 0x4C , 0x24 , 0x08 , 0x89 , 0x44 , 0x24 , 0x0C , 0x89 , 0x0C , 0x24 , 0x89 , 0x44 , 0x24 , 0x04 , 0x89 , 0xE0 , 0x31 , 0xDB , 0xCD , 0x80 , 0x83 , 0xC4 , 0x10 , 0x5B , 0xC3 , 0x6C , 0x6F , 0x6C , 0x20 , 0x70 , 0x6C , 0x65 , 0x61 , 0x73 , 0x65 , 0x20 , 0x77 , 0x6F , 0x72 , 0x6B , 0x01 , 0x67 , 0x64 , 0x62 , 0x5F , 0x6C , 0x6F , 0x61 , 0x64 , 0x5F , 0x72 , 0x75 , 0x73 , 0x74 , 0x5F , 0x70 , 0x72 , 0x65 , 0x74 , 0x74 , 0x79 , 0x5F , 0x70 , 0x72 , 0x69 , 0x6E , 0x74 , 0x65 , 0x72 , 0x73 , 0x2E , 0x70 , 0x79 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x80 , 0x80 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x03 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0xD6 , 0x80 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x03 , 0x00 , 0x02 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0xE5 , 0x80 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x03 , 0x00 , 0x03 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x04 , 0x00 , 0xF1 , 0xFF , 0x12 , 0x00 , 0x00 , 0x00 , 0xB0 , 0x80 , 0x04 , 0x08 , 0x26 , 0x00 , 0x00 , 0x00 , 0x02 , 0x00 , 0x01 , 0x00 , 0x45 , 0x00 , 0x00 , 0x00 , 0xA0 , 0x80 , 0x04 , 0x08 , 0x06 , 0x00 , 0x00 , 0x00 , 0x12 , 0x02 , 0x01 , 0x00 , 0x5C , 0x00 , 0x00 , 0x00 , 0x80 , 0x80 , 0x04 , 0x08 , 0x1D , 0x00 , 0x00 , 0x00 , 0x12 , 0x00 , 0x01 , 0x00 , 0x57 , 0x00 , 0x00 , 0x00 , 0x07 , 0x91 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x03 , 0x00 , 0x63 , 0x00 , 0x00 , 0x00 , 0xE5 , 0x80 , 0x04 , 0x08 , 0x22 , 0x00 , 0x00 , 0x00 , 0x21 , 0x00 , 0x03 , 0x00 , 0x87 , 0x00 , 0x00 , 0x00 , 0x07 , 0x91 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x03 , 0x00 , 0x8E , 0x00 , 0x00 , 0x00 , 0x08 , 0x91 , 0x04 , 0x08 , 0x00 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x03 , 0x00 , 0x00 , 0x34 , 0x38 , 0x6B , 0x76 , 0x66 , 0x34 , 0x35 , 0x39 , 0x68 , 0x78 , 0x6F , 0x76 , 0x38 , 0x37 , 0x6D , 0x6D , 0x00 , 0x5F , 0x5A , 0x4E , 0x31 , 0x32 , 0x6F , 0x73 , 0x73 , 0x69 , 0x5F , 0x70 , 0x72 , 0x6F , 0x67 , 0x72 , 0x61 , 0x6D , 0x37 , 0x50 , 0x72 , 0x69 , 0x6E , 0x74 , 0x4C , 0x6E , 0x34 , 0x63 , 0x61 , 0x6C , 0x6C , 0x31 , 0x37 , 0x68 , 0x66 , 0x64 , 0x39 , 0x36 , 0x30 , 0x36 , 0x32 , 0x33 , 0x34 , 0x39 , 0x62 , 0x63 , 0x30 , 0x39 , 0x33 , 0x65 , 0x45 , 0x00 , 0x72 , 0x75 , 0x73 , 0x74 , 0x5F , 0x62 , 0x65 , 0x67 , 0x69 , 0x6E , 0x5F , 0x75 , 0x6E , 0x77 , 0x69 , 0x6E , 0x64 , 0x00 , 0x5F , 0x5F , 0x62 , 0x73 , 0x73 , 0x5F , 0x73 , 0x74 , 0x61 , 0x72 , 0x74 , 0x00 , 0x5F , 0x5F , 0x72 , 0x75 , 0x73 , 0x74 , 0x63 , 0x5F , 0x64 , 0x65 , 0x62 , 0x75 , 0x67 , 0x5F , 0x67 , 0x64 , 0x62 , 0x5F , 0x73 , 0x63 , 0x72 , 0x69 , 0x70 , 0x74 , 0x73 , 0x5F , 0x73 , 0x65 , 0x63 , 0x74 , 0x69 , 0x6F , 0x6E , 0x5F , 0x5F , 0x00 , 0x5F , 0x65 , 0x64 , 0x61 , 0x74 , 0x61 , 0x00 , 0x5F , 0x65 , 0x6E , 0x64 , 0x00 , 0x00 , 0x2E , 0x73 , 0x79 , 0x6D , 0x74 , 0x61 , 0x62 , 0x00 , 0x2E , 0x73 , 0x74 , 0x72 , 0x74 , 0x61 , 0x62 , 0x00 , 0x2E , 0x73 , 0x68 , 0x73 , 0x74 , 0x72 , 0x74 , 0x61 , 0x62 , 0x00 , 0x2E , 0x74 , 0x65 , 0x78 , 0x74 , 0x00 , 0x2E , 0x72 , 0x6F , 0x64 , 0x61 , 0x74 , 0x61 , 0x00 , 0x2E , 0x64 , 0x65 , 0x62 , 0x75 , 0x67 , 0x5F , 0x67 , 0x64 , 0x62 , 0x5F , 0x73 , 0x63 , 0x72 , 0x69 , 0x70 , 0x74 , 0x73 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x1B , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x06 , 0x00 , 0x00 , 0x00 , 0x80 , 0x80 , 0x04 , 0x08 , 0x80 , 0x00 , 0x00 , 0x00 , 0x56 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x21 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x02 , 0x00 , 0x00 , 0x00 , 0xD6 , 0x80 , 0x04 , 0x08 , 0xD6 , 0x00 , 0x00 , 0x00 , 0x0F , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x29 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x32 , 0x00 , 0x00 , 0x00 , 0xE5 , 0x80 , 0x04 , 0x08 , 0xE5 , 0x00 , 0x00 , 0x00 , 0x22 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x02 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x08 , 0x01 , 0x00 , 0x00 , 0xC0 , 0x00 , 0x00 , 0x00 , 0x05 , 0x00 , 0x00 , 0x00 , 0x06 , 0x00 , 0x00 , 0x00 , 0x04 , 0x00 , 0x00 , 0x00 , 0x10 , 0x00 , 0x00 , 0x00 , 0x09 , 0x00 , 0x00 , 0x00 , 0x03 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0xC8 , 0x01 , 0x00 , 0x00 , 0x93 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x11 , 0x00 , 0x00 , 0x00 , 0x03 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x5B , 0x02 , 0x00 , 0x00 , 0x3C , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x01 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00 , 0x00]);
        // syscall::PrintLn::call("Lol");

    }

    loop {
        syscall::Halt::call();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
