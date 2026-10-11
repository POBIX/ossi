#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    Gray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode(((background as u8) << 4) | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Char {
    pub(crate) byte: u8,
    pub(crate) color: ColorCode,
}

pub const VGA_BUFFER_WIDTH: usize = 80;
pub const VGA_BUFFER_HEIGHT: usize = 15;
pub const VGA_BUFFER_SIZE: usize = VGA_BUFFER_WIDTH * VGA_BUFFER_HEIGHT;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
#[cfg(feature = "kernel")]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    let _guard = crate::kernel::interrupts::KernelInterruptGuard::new();
    let result = {
        let mut console = crate::kernel::vga_console::CONSOLE.lock();
        console.write_fmt(args)
    };
    result.unwrap()
        
}

#[doc(hidden)]
#[cfg(not(feature = "kernel"))]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    use alloc::string::String;

    let _guard = crate::interrupts::InterruptGuard::new();

    let mut formatted = String::new();
    formatted.write_fmt(args).unwrap();

    crate::syscall::Print::call(formatted.as_ptr(), formatted.len());
}
