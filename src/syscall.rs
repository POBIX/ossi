use crate::paging::PageFlags;
use core::mem::MaybeUninit;
use crate::fs::{File, FileError};
use crate::vga_console::ColorCode;
use core::arch::asm; macro_rules! decl_syscall {
    (_internal $name: ident = $func: path {$($arg_name:ident : $arg_type:ty),*} -> $ret_type:ty) => {
        #[allow(dead_code)]
        pub struct $name {
            $($arg_name: $arg_type,)*
            _ret: MaybeUninit<$ret_type>
        }

        impl $name {
            pub fn call($($arg_name: $arg_type),*) -> $ret_type {
                // this should keep existing until the end of the function (after the syscall)
                let s = $name { $($arg_name,)* _ret: MaybeUninit::uninit() };
                unsafe {
                    asm!("int 0x80", in("eax") &s, in("ebx") Syscall::$name as u32);
                    s._ret.assume_init()
                }
            }

            #[cfg(feature = "kernel")]
            #[allow(unused_unsafe)] // unsafe in case $func is unsafe.
            fn call_internal(&self) -> $ret_type { unsafe { $func($(self.$arg_name),*) } }
        }
    };
    (_internal $name: ident = $func: path {$($arg_name:ident : $arg_type:ty),*}) => {
        decl_syscall!{_internal $name = $func {$($arg_name : $arg_type),*} -> ()}
    };
}

macro_rules! decl_syscalls {
    ( $( $name:ident = $syscall:path { $( $param:ident : $ty:ty ),* } $(-> $ret:ty)? ),* ) => {
        $(decl_syscall!(_internal $name = $syscall { $( $param : $ty ),* } $(-> $ret)?);)*

        #[repr(u32)]
        pub enum Syscall {
            $( $name, )*
        }

        #[cfg(feature = "kernel")]
        #[no_mangle]
        extern "C" fn syscall_handler_inner(syscall: u32, ty: Syscall) {
            match ty {
                $( Syscall::$name => { 
                    let s = unsafe { core::mem::transmute::<u32, &mut $name>(syscall) };
                    s._ret.write(s.call_internal());
                })*
            }
        }
    };
}

decl_syscalls!(
    Print = crate::kernel::syscall::print{str: *const u8, len: usize},
    AreInterruptsEnabled = crate::kernel::interrupts::is_enabled{} -> bool,
    DisableInterrupts = crate::kernel::interrupts::disable{},
    Halt = crate::kernel::syscall::halt{},
    Empty = crate::kernel::syscall::empty{},
    Outb = crate::kernel::io::outb{port: u16, value: u8},
    Outw = crate::kernel::io::outw{port: u16, value: u16},
    Outl = crate::kernel::io::outl{port: u16, value: u32},
    EnableInterrupts = crate::kernel::interrupts::enable{},
    ReadSectors = crate::kernel::ata::read_sectors{lba: u32, buffer: *mut u8, sector_count: usize},
    WriteSectors = crate::kernel::ata::write_sectors{lba: u32, data: *const u8, sector_count: usize},
    PicSetMask = crate::kernel::pic::set_mask{irq_line: u8, value: bool},
    MapPages = crate::kernel::syscall::map_pages{n: usize, flags: PageFlags} -> usize,
    UnmapPage = crate::kernel::syscall::unmap_page{page: usize},
    OpenFile = crate::kernel::fs::open_file{path: *const u8, len: usize} -> Result<File, FileError>,
    CreateFile = crate::kernel::fs::create_file{path: *const u8, len: usize} -> Result<File, FileError>,
    DeleteFile = crate::kernel::fs::delete_file{fd: usize} -> Result<(), FileError>,
    CloseFile = crate::kernel::fs::close_file{fd: usize},
    GetFileMetadata = crate::kernel::fs::get_metadata{fd: usize} -> crate::fs::FileMetadata,
    ConsoleWriteByte = crate::kernel::syscall::write_byte{byte: u8},
    ConsoleWriteString = crate::kernel::syscall::write_string{ptr: *const u8, len: usize},
    ConsoleReadByte = crate::kernel::syscall::read_byte{} -> u8,
    ConsoleSeek = crate::kernel::syscall::seek{pos: usize},
    ConsoleGetCursorPosition = crate::kernel::syscall::get_cursor_position{} -> usize,
    ConsoleSeekRaw = crate::kernel::syscall::seek_raw{pos: usize},
    ConsoleUpdateCursor = crate::kernel::syscall::update_cursor{},
    ConsoleNewline = crate::kernel::syscall::newline{},
    ConsoleNewlineRaw = crate::kernel::syscall::newline_raw{},
    ConsoleBackspace = crate::kernel::syscall::backspace{},
    ConsoleClear = crate::kernel::syscall::clear{},
    ConsoleSetColor = crate::kernel::syscall::set_color{color: ColorCode},
    ConsoleGetColor = crate::kernel::syscall::get_color{} -> ColorCode
);
