#![feature(abi_x86_interrupt)]
#![feature(once_cell)]
#![no_std]
#![no_main]

use crate::interrupts::GateType;
use crate::io::Clear;
use crate::vga_console::CONSOLE;
use core::arch::asm;
use core::panic::PanicInfo;

pub mod interrupts;
pub mod io;
pub mod pic;
pub mod vga_console;

#[no_mangle]
pub extern "C" fn main() -> ! {
    CONSOLE.lock().clear();
    pic::remap();
    unsafe {
        interrupts::IDT[0x20] = interrupts::Handler::new(irq0, GateType::DInterrupt);
        interrupts::IDT[0x21] = interrupts::Handler::new(irq1, GateType::DInterrupt);
    }
    interrupts::init();

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

static mut LOL: u8 = b'a';

extern "x86-interrupt" fn irq0() {
    unsafe {
        print!("{}", LOL as char);
        LOL += 1;
        if LOL >= b'z' {
            LOL = b'a';
        }
    }

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
