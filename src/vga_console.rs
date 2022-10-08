use crate::io;
use crate::io::{Write, Seek, Clear};

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
    pub(crate) const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode(((background as u8) << 4) | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct Char {
    byte: u8,
    color: ColorCode,
}

const VGA_BUFFER_SIZE: usize = 80 * 25; // width * height

#[repr(transparent)]
struct Buffer { buffer: [Char; VGA_BUFFER_SIZE] }
impl Buffer {
    fn write(&mut self, ptr: usize, byte: u8, color: ColorCode) {
        self.buffer[ptr] = Char { byte, color };
    }
}

pub struct Console {
    ptr: usize,
    buffer: &'static mut Buffer,
    color: ColorCode,
}

impl Write for Console {
    fn write_byte(&mut self, byte: u8) {
        self.buffer.write(self.ptr, byte, self.color);
        self.ptr += 1;
        self.update_cursor();
    }
    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.buffer.write(self.ptr, *byte, self.color);
            self.ptr += 1;
        }
        self.update_cursor();
    }
}

impl Seek for Console {
    fn seek(&mut self, pos: usize) {
        self.ptr = pos;
        self.update_cursor()
    }

    #[inline]
    fn get_cursor_position(&self) -> usize { self.ptr }
}

impl Clear for Console {
    fn clear(&mut self) {
        self.seek_raw(0);
        for i in 0..VGA_BUFFER_SIZE {
            self.buffer.write(i, b' ', self.color);
        }
        self.seek(0);
    }
}

impl Console {
    #[inline]
    pub fn set_color(&mut self, color: ColorCode) { self.color = color; }
    #[inline]
    pub fn get_color(&self) -> ColorCode { self.color }

    pub fn update_cursor(&self) {
        unsafe {
            // 0x3D4/5 - the I/O ports for the VGA cursor
            // we send each byte of the cursor position in turn.
            io::outb(0x3D4, 0x0E);
            io::outb(0x3D5, ((self.ptr >> 8) & 0xFF) as u8);
            io::outb(0x3D4, 0x0F);
            io::outb(0x3D5, (self.ptr & 0xFF) as u8);
        }
    }

    /// Set the cursor's logical position without updating its visual position.
    #[inline]
    pub fn seek_raw(&mut self, value: usize) { self.ptr = value; }

    pub fn new() -> Console {
        Console {
            ptr: 0,
            color: ColorCode::new(Color::White, Color::Black),
            buffer: unsafe { &mut *(0xB8000 as *mut Buffer) } // address of VGA text buffer
        }
    }
}
