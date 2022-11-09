use core::arch::asm;

pub trait Write {
    fn write_byte(&mut self, byte: u8);
    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.write_byte(*byte);
        }
    }
    fn write_string(&mut self, str: &str) {
        for byte in str.bytes() {
            match byte {
                0x20..=0x7E => self.write_byte(byte), // writable ASCII. space to ~
                _ => self.write_byte(b'?')
            }
        }
    }
}

pub trait Read {
    fn read_byte(&self) -> u8;
    // fn read_bytes(&self, count: u64) -> Vec<u8> {
    //     let mut out: Vec<u8> = vec![];
    //     for _ in 0..count {
    //         out.push(self.read_byte());
    //     }
    //     out
    // }
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
