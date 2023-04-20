use core::arch::asm;
use crate::{interrupts, io::Write};
// No variadic generics :(
pub trait SyscallBase {
    fn call_internal(&self);
}

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

        impl SyscallBase for $name {
            // unsafe in case $func is unsafe.
            #[allow(unused_unsafe)]
            fn call_internal(&self) { unsafe { $func($(self.$arg_name),*); } }
        }

        impl $name {
            pub fn call($($arg_name: $arg_type),*) {
                // this should keep existing until the end of the function (after the syscall)
                let s = $name { $($arg_name),* };
                let r = &s as &dyn SyscallBase;
                let (ptr_low, ptr_high): (u32, u32) = unsafe { core::mem::transmute(r) };
                unsafe {
                    asm!("int 0x80", in("eax") ptr_high, in("ebx") ptr_low);
                }
            }
        }
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
            // We pass the syscall (a fat pointer) as an argument to the inner function,
            // which simply transmutes the numbers into the syscall object and executes the syscall.
            "push eax",
            "push ebx",
            "call syscall_handler_inner",
            "add esp, 8" // pop eax and ebx
        )
    }
}

#[no_mangle]
extern "C" fn syscall_handler_inner(syscall: &dyn SyscallBase) {
    syscall.call_internal();
}

/* Definition of all specific syscalls */

decl_syscall!(PrintLn = println_syscall{msg: &'static str});
decl_syscall!(DisableInterrupts = crate::interrupts::disable{});
decl_syscall!(Halt = halt{});
decl_syscall!(Alloc = alloc{ptr: *mut *mut u8, layout: core::alloc::Layout});
decl_syscall!(Dealloc = alloc::alloc::dealloc{ptr: *mut u8, layout: core::alloc::Layout});
decl_syscall!(Empty = empty{});

fn println_syscall(msg: &str) {
    crate::vga_console::CONSOLE.lock().write_string(msg);
}

unsafe fn alloc(ptr: *mut *mut u8, layout: core::alloc::Layout) {
    *ptr = alloc::alloc::alloc(layout);
}

fn empty() {}

fn halt() { unsafe { asm!("hlt"); } }
