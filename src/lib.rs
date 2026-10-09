#![feature(abi_x86_interrupt)]
#![feature(pointer_byte_offsets)]
#![feature(int_roundings)]
#![feature(const_mut_refs)]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(new_uninit)]
#![feature(inline_const)]
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
pub mod ata;
pub mod fs;
pub mod execution;
pub mod paging;
mod userspace;
pub mod syscall;
pub mod process;

extern "C" {
    static CODE_SEG: usize;
    static DATA_SEG: usize;
    static KERNEL_STACK_TOP: usize;
}

#[no_mangle]
pub(crate) extern "C" fn kernel_main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();

    pic::remap();
    timer::init();
    interrupts::init();


    userspace::init();
    syscall::init();

    paging::init();

    CONSOLE.lock().clear();
    keyboard::init();
    ata::init();

    execution::execute_file(&mut fs::File::open("/shell").unwrap());

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
