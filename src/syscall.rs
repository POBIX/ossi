use core::arch::asm;
use alloc::{vec::Vec, string::String};
use spin::{Mutex, Lazy};

use crate::events::Event;
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
            #[cfg(feature = "kernel")]
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
            #[cfg(feature = "kernel")]
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

        #[cfg(feature = "kernel")]
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

        #[cfg(feature = "kernel")]
        #[no_mangle]
        extern "C" fn syscall_handler_inner_lifetime<'a>(syscall: u32, ty: SyscallLifetime) {
            match ty {
                $( SyscallLifetime::$name => unsafe{core::mem::transmute::<u32, &mut $name<'a>>(syscall)}.call_internal(), )*
            }
        }
    };
}

/* Definition of all specific syscalls */

decl_syscalls!(
    Print<'a> = crate::kernel::syscall::print_syscall{args: core::fmt::Arguments<'a>},
    AreInterruptsEnabled<'a> = crate::kernel::syscall::are_interrupts_enabled{value: &'a mut bool},
    RunProgram<'a> = crate::kernel::execution::run_program{program: &'a [u8]},
    HasLoadedProcesses<'a> = crate::kernel::syscall::has_loaded_processes{out: &'a mut bool},
    GetCurrPageDir<'a> = crate::kernel::syscall::get_curr_page_dir{out: &'a mut *mut crate::paging::PageDirectory},
    GetOnKeyDown<'a> = crate::kernel::syscall::get_on_key_down{out: &'a mut &'static Mutex<Event<crate::keyboard::KeyArgs>>},
    GetOnKeyUp<'a> = crate::kernel::syscall::get_on_key_up{out: &'a mut &'static Mutex<Event<crate::keyboard::KeyArgs>>},
    GetConsole<'a> = crate::kernel::syscall::get_console{out: &'a mut &'static Lazy<Mutex<crate::vga_console::Console>>},
    IsKeyPressed<'a> = crate::kernel::syscall::is_key_pressed{out: &'a mut bool, key: crate::keyboard::Key},
    IsCapsLockActive<'a> = crate::kernel::syscall::is_caps_lock_active{out: &'a mut bool},
    FsGetHeader<'a> = crate::kernel::syscall::fs_get_header{out: &'a mut &'static Lazy<Mutex<&'static mut crate::fs::Header>>},
    GetFilesInDir<'a> = crate::kernel::fs::dir{root: &'a String, folders: &'a mut Vec<String>, files: &'a mut Vec<crate::fs::FileMetadata>},
    ExecuteFile<'a> = crate::kernel::execution::execute_file{file: &'a mut crate::fs::File}
);
decl_syscalls!(
    DisableInterrupts = crate::kernel::interrupts::disable{},
    EnableInterrupts = crate::kernel::interrupts::enable{},
    Halt = crate::kernel::syscall::halt{},
    Empty = crate::kernel::syscall::empty{},
    Outb = crate::kernel::io::outb{port: u16, value: u8},
    Outw = crate::kernel::io::outw{port: u16, value: u16},
    Outl = crate::kernel::io::outl{port: u16, value: u32},
    ReadSectors = crate::kernel::ata::read_sectors{lba: u32, buffer: *mut u8, sector_count: usize},
    WriteSectors = crate::kernel::ata::write_sectors{lba: u32, data: *const u8, sector_count: usize},
    SetIsr = crate::kernel::syscall::set_isr{index: usize, func: extern "x86-interrupt" fn(), dpl: u8},
    PicSendEoi = crate::kernel::pic::send_eoi{irq_line: u8},
    PicSetMask = crate::kernel::pic::set_mask{irq_line: u8, value: bool},
    UnreigsterProcess = crate::kernel::process::unregister_prev{},
    IoWait = crate::kernel::io::wait{}
);

#[allow(invalid_value)] // out's initial value is discarded. Giving it an actual value would be a massive waste of performance.
pub fn get_fs_header() -> &'static Lazy<Mutex<&'static mut crate::fs::Header>>{
    let mut out = unsafe { core::mem::transmute(0) };
    FsGetHeader::call(&mut out);
    out
}

