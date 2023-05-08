use core::arch::asm;
use crate::{interrupts, io::Write};
// No variadic generics :(
pub trait SyscallBase {}

pub trait Syscall0: SyscallBase {}

pub trait Syscall1: SyscallBase {
    type T1;
}

pub trait Syscall2: SyscallBase {
    type T1;
    type T2;
}

pub trait Syscall3: SyscallBase {
    type T1;
    type T2;
    type T3;
}

pub trait Syscall4: SyscallBase {
    type T1;
    type T2;
    type T3;
    type T4;
}

#[macro_export]
macro_rules! decl_syscall {
    (_internal $name: ident = $func: path [$($arg_name:ident : $arg_type:ty),*]) => {
        pub struct $name {
            $($arg_name: $arg_type),*
        }

        impl $name {
            pub fn call($($arg_name: $arg_type),*) {
                // this should keep existing until the end of the function (after the syscall)
                let s = $name { $($arg_name),* };
                unsafe {
                    asm!("int 0x80", in("eax") &s, in("ebx") Syscall::$name as u32);
                }
            }

            // unsafe in case $func is unsafe.
            #[allow(unused_unsafe)]
            fn call_internal(&self) { unsafe { $func($(self.$arg_name),*); } }
        }

        impl SyscallBase for $name {}
    };

    ($name: ident = $func: path {}) => {
        decl_syscall!(_internal $name = $func[]);
        impl Syscall0 for $name {}
    };
    ($name: ident = $func: path {$arg1n: ident: $arg1t: ty}) => {
        decl_syscall!(_internal $name = $func[$arg1n: $arg1t]);
        impl Syscall1 for $name {
            type T1 = $arg1t;
        }
    };
    ($name: ident = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty}) => {
        decl_syscall!(_internal $name = $func[$arg1n: $arg1t, $arg2n: $arg2t]);
        impl Syscall2 for $name {
            type T1 = $arg1t;
            type T2 = $arg2t;
        }
    };
    ($name: ident = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty, $arg3n: ident: $arg3t: ty}) => {
        decl_syscall!(_internal $name = $func[$arg1n: $arg1t, $arg2n: $arg2t, $arg3n: $arg3t]);
        impl Syscall3 for $name {
            type T1 = $arg1t;
            type T2 = $arg2t;
            type T3 = $arg3t;
        }
    };
    ($name: ident = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty, $arg3n: ident: $arg3t: ty, $arg4n: ident: $arg4t: ty}) => {
        decl_syscall!(_internal $name = $func[$arg1n: $arg1t, $arg2n: $arg2t, $arg3n: $arg3t, $arg4n: $arg4t]);
        impl Syscall4 for $name {
            type T1 = $arg1t;
            type T2 = $arg2t;
            type T3 = $arg3t;
            type T4 = $arg4t;
        }
    };
}

macro_rules! decl_syscalls {
    ( $( $name:ident = $syscall:path { $( $param:ident : $ty:ty ),* } ),* ) => {
        $(decl_syscall!($name = $syscall { $( $param : $ty ),* });)*

        #[repr(u32)]
        pub enum Syscall {
            $( $name, )*
        }

        #[no_mangle]
        extern "C" fn syscall_handler_inner(syscall: u32, ty: Syscall) {
            match ty {
                $( Syscall::$name => unsafe{core::mem::transmute::<u32, &$name>(syscall)}.call_internal(), )*
            }
        }
    };
}

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(
            syscall_handler as *const () as u32, interrupts::GateType::DInterrupt, 3
        );
    }
}

extern "x86-interrupt" fn syscall_handler() {
    unsafe {
        asm!(
            "cli",
            "push ebx",
            "push eax",
            "call syscall_handler_inner",
            "add esp, 8", // pop the parameters
            "sti"
        )
    }
}

/* Definition of all specific syscalls */

decl_syscalls!(
    PrintLn = println_syscall{msg: &'static str},
    DisableInterrupts = crate::interrupts::disable{},
    EnableInterrupts = crate::interrupts::enable{},
    AreInterruptsEnabled = are_interrupts_enabled{value: *mut bool},
    Halt = halt{},
    Alloc = alloc{ptr: *mut *mut u8, layout: core::alloc::Layout},
    Dealloc = alloc::alloc::dealloc{ptr: *mut u8, layout: core::alloc::Layout},
    Empty = empty{},
    Outb = crate::io::outb{port: u16, value: u8},
    Outw = crate::io::outw{port: u16, value: u16},
    Outl = crate::io::outl{port: u16, value: u32},
    NextProgram = crate::process::next_program{new_context: *const crate::process::Context, after: fn()}
);

fn println_syscall(msg: &str) {
    crate::vga_console::CONSOLE.lock().write_string(msg);
}

fn are_interrupts_enabled(value: *mut bool) {
    unsafe { *value = crate::interrupts::is_enabled() };
}

unsafe fn alloc(ptr: *mut *mut u8, layout: core::alloc::Layout) {
    *ptr = alloc::alloc::alloc(layout);
}

fn empty() {}

fn halt() { unsafe { asm!("hlt"); } }
