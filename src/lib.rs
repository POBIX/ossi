#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![no_std]
#![no_main]

use crate::interrupts::{GateType, Handler, StackInterruptFrame};
use crate::io::Clear;
use crate::vga_console::CONSOLE;
use core::arch::asm;
use core::panic::PanicInfo;

pub mod interrupts;
pub mod io;
pub mod vga_console;

#[no_mangle]
pub extern "C" fn main() -> ! {
    CONSOLE.lock().clear();

    unsafe {
        interrupts::IDT[0x3] = Handler::new(breakpoint, GateType::DInterrupt);
        interrupts::init();
        asm!("int3");
    }
    loop {}
}

extern "x86-interrupt" fn breakpoint(_: &StackInterruptFrame) {
    println!("breakpoint reached!");
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
