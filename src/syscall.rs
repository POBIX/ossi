use core::arch::asm;
use alloc::{vec::Vec, string::String};
use spin::{Mutex, Lazy};

use crate::{interrupts, events::Event};
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

    (_internal $name: ident<'a> = $func: path [$($arg_name:ident : $arg_type:ty),*]) => {
        pub struct $name<'a> {
            $($arg_name: $arg_type),*
        }

        impl<'a> $name<'a> {
            pub fn call($($arg_name: $arg_type),*) {
                // this should keep existing until the end of the function (after the syscall)
                let s = $name { $($arg_name),* };
                unsafe {
                    asm!("int 0x81", in("eax") &s, in("ebx") SyscallLifetime::$name as u32);
                }
            }

            // unsafe in case $func is unsafe.
            #[allow(unused_unsafe)]
            fn call_internal(&mut self) { unsafe { $func($(self.$arg_name),*); } }
        }

        impl<'a> SyscallBase for $name<'a> {}
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

    ($name: ident<'a> = $func: path {}) => {
        decl_syscall!(_internal $name<'a> = $func[]);
        impl<'a> Syscall0 for $name<'a> {}
    };
    ($name: ident<'a> = $func: path {$arg1n: ident: $arg1t: ty}) => {
        decl_syscall!(_internal $name<'a> = $func[$arg1n: $arg1t]);
        impl<'a> Syscall1 for $name<'a> {
            type T1 = $arg1t;
        }
    };
    ($name: ident<'a> = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty}) => {
        decl_syscall!(_internal $name<'a> = $func[$arg1n: $arg1t, $arg2n: $arg2t]);
        impl<'a> Syscall2 for $name<'a> {
            type T1 = $arg1t;
            type T2 = $arg2t;
        }
    };
    ($name: ident<'a> = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty, $arg3n: ident: $arg3t: ty}) => {
        decl_syscall!(_internal $name<'a> = $func[$arg1n: $arg1t, $arg2n: $arg2t, $arg3n: $arg3t]);
        impl<'a> Syscall3 for $name<'a> {
            type T1 = $arg1t;
            type T2 = $arg2t;
            type T3 = $arg3t;
        }
    };
    ($name: ident<'a> = $func: path {$arg1n: ident: $arg1t: ty, $arg2n: ident: $arg2t: ty, $arg3n: ident: $arg3t: ty, $arg4n: ident: $arg4t: ty}) => {
        decl_syscall!(_internal $name<'a> = $func[$arg1n: $arg1t, $arg2n: $arg2t, $arg3n: $arg3t, $arg4n: $arg4t]);
        impl<'a> Syscall4 for $name<'a> {
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

    ( $( $name:ident<'a> = $syscall:path { $( $param:ident : $ty:ty ),* } ),* ) => {
        $(decl_syscall!($name<'a> = $syscall { $( $param : $ty ),* });)*

        #[repr(u32)]
        pub enum SyscallLifetime {
            $( $name, )*
        }

        #[no_mangle]
        extern "C" fn syscall_handler_inner_lifetime<'a>(syscall: u32, ty: SyscallLifetime) {
            match ty {
                $( SyscallLifetime::$name => unsafe{core::mem::transmute::<u32, &mut $name<'a>>(syscall)}.call_internal(), )*
            }
        }
    };
}

pub fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new_raw(
            syscall_handler as *const () as u32, interrupts::GateType::DInterrupt, 3
        );
        interrupts::IDT[0x81] = interrupts::Handler::new_raw(
            syscall_handler_lifetime as *const () as u32, interrupts::GateType::DInterrupt, 3
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

extern "x86-interrupt" fn syscall_handler_lifetime() {
    unsafe {
        asm!(
            "cli",
            "push ebx",
            "push eax",
            "call syscall_handler_inner_lifetime",
            "add esp, 8", // pop the parameters
            "sti"
        )
    }
}

/* Definition of all specific syscalls */

decl_syscalls!(
    Print<'a> = print_syscall{args: core::fmt::Arguments<'a>},
    AreInterruptsEnabled<'a> = are_interrupts_enabled{value: &'a mut bool},
    Alloc<'a> = alloc{ptr: &'a mut *mut u8, layout: core::alloc::Layout},
    RunProgram<'a> = crate::execution::run_program{program: &'a [u8]},
    HasInitHeap<'a> = has_init_heap{out: &'a mut bool},
    HasLoadedProcesses<'a> = has_loaded_processes{out: &'a mut bool},
    GetCurrPageDir<'a> = get_curr_page_dir{out: &'a mut *mut crate::paging::PageDirectory},
    GetOnKeyDown<'a> = get_on_key_down{out: &'a mut &'static Mutex<Event<crate::keyboard::KeyArgs>>},
    GetOnKeyUp<'a> = get_on_key_up{out: &'a mut &'static Mutex<Event<crate::keyboard::KeyArgs>>},
    GetConsole<'a> = get_console{out: &'a mut &'static Lazy<Mutex<crate::vga_console::Console>>},
    IsKeyPressed<'a> = is_key_pressed{out: &'a mut bool, key: crate::keyboard::Key},
    IsCapsLockActive<'a> = is_caps_lock_active{out: &'a mut bool},
    FsGetHeader<'a> = fs_get_header{out: &'a mut &'static Lazy<Mutex<&'static mut crate::fs::Header>>},
    GetFilesInDir<'a> = crate::fs::dir{root: &'a String, folders: &'a mut Vec<String>, files: &'a mut Vec<crate::fs::FileMetadata>},
    ExecuteFile<'a> = crate::execution::execute_file{file: &'a mut crate::fs::File}
);
decl_syscalls!(
    DisableInterrupts = crate::interrupts::disable{},
    EnableInterrupts = crate::interrupts::enable{},
    Halt = halt{},
    Empty = empty{},
    Outb = crate::io::outb{port: u16, value: u8},
    Outw = crate::io::outw{port: u16, value: u16},
    Outl = crate::io::outl{port: u16, value: u32},
    ReadSectors = crate::ata::read_sectors{lba: u32, buffer: *mut u8, sector_count: usize},
    WriteSectors = crate::ata::write_sectors{lba: u32, data: *const u8, sector_count: usize},
    SetIsr = set_isr{index: usize, func: extern "x86-interrupt" fn(), dpl: u8},
    PicSendEoi = crate::pic::send_eoi{irq_line: u8},
    PicSetMask = crate::pic::set_mask{irq_line: u8, value: bool},
    Dealloc = dealloc{ptr: *mut u8, layout: core::alloc::Layout}
);

fn print_syscall(args: core::fmt::Arguments) {
    crate::vga_console::_print(args);
}

unsafe fn alloc(ptr: &mut *mut u8, layout: core::alloc::Layout) {
    *ptr = crate::heap::HEAP.alloc_internal(layout);
}

unsafe fn dealloc(ptr: *mut u8, layout: core::alloc::Layout) {
    crate::heap::HEAP.dealloc_internal(ptr, layout);
}

fn is_key_pressed(out: &mut bool, key: crate::keyboard::Key) {
    *out = crate::keyboard::is_key_pressed(key);
}

fn empty() {}

fn halt() { unsafe { asm!("hlt"); } }

unsafe fn set_isr(index: usize, func: extern "x86-interrupt" fn(), dpl: u8) {
    crate::interrupts::IDT[index] = crate::interrupts::Handler::new(func, interrupts::GateType::DInterrupt, dpl);
}

macro_rules! generate_ret_func {
    ($name: ident, $expr: expr, $type: ty) => {
        fn $name(out: &mut $type) {
            *out = $expr;
        }
    };
}

generate_ret_func!(has_init_heap, crate::heap::has_init(), bool);
generate_ret_func!(has_loaded_processes, crate::process::has_loaded_processes(), bool);
generate_ret_func!(get_curr_page_dir, crate::paging::PageDirectory::curr(), *mut crate::paging::PageDirectory);
generate_ret_func!(are_interrupts_enabled, crate::interrupts::is_enabled(), bool);
generate_ret_func!(get_on_key_down, &crate::keyboard::ON_KEY_DOWN, &'static Mutex<Event<crate::keyboard::KeyArgs>>);
generate_ret_func!(get_on_key_up, &crate::keyboard::ON_KEY_UP, &'static Mutex<Event<crate::keyboard::KeyArgs>>);
generate_ret_func!(get_console, &crate::vga_console::CONSOLE, &'static Lazy<Mutex<crate::vga_console::Console>>);
generate_ret_func!(is_caps_lock_active, crate::keyboard::is_caps_lock_active(), bool);
generate_ret_func!(fs_get_header, &crate::fs::HEADER, &'static Lazy<Mutex<&'static mut crate::fs::Header>>);

#[allow(invalid_value)] // out's initial value is discarded. Giving it an actual value would be a massive waste of performance.
pub fn get_fs_header() -> &'static Lazy<Mutex<&'static mut crate::fs::Header>>{
    let mut out = unsafe { core::mem::transmute(0) };
    FsGetHeader::call(&mut out);
    out
}
