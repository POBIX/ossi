#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![no_std]
#![no_main]

use crate::io::Clear;
use crate::vga_console::CONSOLE;
use core::panic::PanicInfo;
use crate::interrupts::GateType;
use core::arch::asm;

pub mod interrupts;
pub mod io;
pub mod vga_console;
pub mod pic;

#[no_mangle]
pub extern "C" fn main() -> ! {
    CONSOLE.lock().clear();
    pic::remap();
    unsafe {
        interrupts::IDT[0x20] = interrupts::Handler::new(irq0, GateType::DInterrupt);
        interrupts::IDT[0x21] = interrupts::Handler::new(irq1, GateType::DInterrupt);
    }
    interrupts::init();

    // never print anything above this comment.

    for _ in 0..1000 {
        print!("-");
    }

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn irq0() {
    print!(".");
    pic::send_eoi(0);
    return;
}

extern "x86-interrupt" fn irq1() {
    print!(",");
    pic::send_eoi(1);
    return;
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
