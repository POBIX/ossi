#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![feature(pointer_byte_offsets)]
#![feature(int_roundings)]
#![feature(const_mut_refs)]
#![feature(asm_const)]
#![feature(naked_functions)]
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

    let heap_start_addr = paging::init();

    unsafe {
        // according to GRUB, there are info.mem_upper free KBs of memory at address 0x100_000.
        // we're using a maximum of a 50MB to get faster loading times,
        // and only start at heap_start_addr since some of the heap was used by paging.
        heap::init(heap_start_addr, core::cmp::min(50 * 1024 * 1024, info.mem_upper));
    }

    userspace::init();
    syscall::init();

    CONSOLE.lock().clear();
    pic::remap();
    timer::init();
    keyboard::init();
    interrupts::init();
    console::init();
    ata::init();

    unsafe {
        println!("{:p} {:p}", &CODE_SEG, &DATA_SEG);
    }

    unsafe {
        userspace::enter();
        syscall::PrintLn::call("lol");
        syscall::DisableInterrupts::call();
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
