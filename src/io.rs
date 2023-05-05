use core::{arch::asm, str::Utf8Error};

use alloc::string::String;

pub trait Write: Seek {
    fn write_byte(&mut self, byte: u8);
    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.write_byte(*byte);
            self.seek(self.get_cursor_position() + 1)
        }
    }
    fn write_string(&mut self, str: &str) {
        self.write_bytes(str.as_bytes());
    }
}

pub trait Read: Seek {
    fn read_byte(&self) -> u8;
    fn read_bytes(&self, buffer: &mut [u8]) -> usize {
        for byte in &mut *buffer {
            *byte = self.read_byte();
        }
        buffer.len()
    }
    fn read_char(&self) -> char { self.read_byte() as char }
    fn read_string(&self, buffer: &mut String) -> Result<usize, Utf8Error> {
        unsafe {
            // Load the string into the buffer
            self.read_bytes(buffer.as_bytes_mut());
        };
        // Check if the string is valid utf8
        let res = core::str::from_utf8(buffer.as_bytes());
        if res.is_err() {
            Err(res.unwrap_err())
        } else {
            Ok(buffer.len())
        }
    }
}

pub trait Seek {
    fn seek(&mut self, pos: usize);
    fn get_cursor_position(&self) -> usize;
}

pub trait Clear {
    fn clear(&mut self);
}

#[inline] pub unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value);
}
#[inline] pub unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value);
}
#[inline] pub unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value);
}

#[inline] pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    asm!("in al, dx", in("dx") port, out("al") ret);
    ret
}
#[inline] pub unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    asm!("in ax, dx", in("dx") port, out("ax") ret);
    ret
}
#[inline] pub unsafe fn inl(port: u16) -> u32 {
    let ret: u32;
    asm!("in eax, dx", in("dx") port, out("eax") ret);
    ret
}

/// wait a very small amount of time by writing to an unused I/O port.
#[inline] pub fn wait() { unsafe { outb(0x80, 0); } }
