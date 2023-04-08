use core::arch::asm;

use crate::{interrupts, println};

static mut SYSCALLS: [fn([u32; 5])->u32; 255] = [|_|{0}; 255]; // an array of function pointers

static mut CALL_IDX: u32 = 0;
static mut ARGS: [u32; 5] = [0; 5];
static mut RET_VAL: u32 = 0;

pub unsafe fn execute(index: u32, args: [u32; 5]) -> u32 {
    CALL_IDX = index;
    ARGS = args;
    asm!("int 0x80");
    RET_VAL
}

#[inline(never)]
fn println_syscall(args: [u32; 5]) -> u32 {
    println!("{}", args[0]);
    0
}

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(syscall_handler as *const () as u32, interrupts::GateType::DInterrupt, 3);
        SYSCALLS[0] = println_syscall;
    }
}

extern "x86-interrupt" fn syscall_handler() {
    unsafe {
        // Since the index is constrained by the enum, this should never be out of bounds
        let func = SYSCALLS[CALL_IDX as usize];
        RET_VAL = func(ARGS);
    }
}
