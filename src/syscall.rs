use core::arch::asm;
use crate::{interrupts, println};

// The address of the current Syscall.
// I'd ideally want a Mutex<Option<&dyn Syscall>> but that insists on a
// static lifetime and using the heap is not an option since no syscalls so
static mut CURR_CALL: u64 = 0;

// No variadic generics :(
pub trait Syscall: Send {
    fn call_internal(&self);
}

pub trait Syscall0: Syscall {}

pub trait Syscall1: Syscall {
    type T1;
}

pub trait Syscall2: Syscall {
    type T1;
    type T2;
}

pub trait Syscall3: Syscall {
    type T1;
    type T2;
    type T3;
}

pub trait Syscall4: Syscall {
    type T1;
    type T2;
    type T3;
    type T4;
}

macro_rules! decl_syscall {
    ($name: ident = $func: ident ($arg1n: ident: $arg1t: ty)) => {
        pub struct $name {
            $arg1n: $arg1t
        }

        impl Syscall for $name {
            fn call_internal(&self) { $func(self.$arg1n) }
        }

        impl Syscall1 for $name {
            type T1 = $arg1t;
        }

        impl $name {
            pub fn call($arg1n: $arg1t) {
                // this should keep existing until the end of the function (after the syscall)
                let s = $name { $arg1n };
                let r = &s as &dyn Syscall;
                unsafe {
                    CURR_CALL = core::mem::transmute(r);
                    asm!("int 0x80");
                }
            }
        }
    };
}

decl_syscall!(PrintLn = println_syscall(msg: &'static str));

#[inline(never)]
fn println_syscall(msg: &str) {
    println!("{}", msg);
}

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(syscall_handler as *const () as u32, interrupts::GateType::DInterrupt, 3);
    }
}

extern "x86-interrupt" fn syscall_handler() {
    let curr: &dyn Syscall = unsafe { core::mem::transmute(CURR_CALL) };
    (*curr).call_internal();
}
