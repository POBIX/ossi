use crate::io;
use crate::io::{Clear, Seek, Write};
use core::fmt;
use spin::{Lazy, Mutex};
use crate::vga_console::*;

//TODO: refactor this mess
impl Buffer {
    fn write(&mut self, ptr: &mut usize, byte: u8, color: ColorCode) {
        while *ptr >= VGA_BUFFER_SIZE {
            self.scroll_down(ptr, color);
        }
        self.buffer[*ptr].write(Char { byte, color });
        *ptr += 1;
    }

    /// scrolls everything one line up, giving us another usable line..
    fn scroll_down(&mut self, ptr: &mut usize, color: ColorCode) {
        // copy each line to the one above it
        for row in 1..VGA_BUFFER_HEIGHT {
            for col in 0..VGA_BUFFER_WIDTH {
                // set the char directly above us to the current char.
                self.buffer[(row - 1) * VGA_BUFFER_WIDTH + col]
                    .write(self.buffer[row * VGA_BUFFER_WIDTH + col].read());
            }
        }
        // clear the last line
        for col in 0..VGA_BUFFER_WIDTH {
            self.buffer[(VGA_BUFFER_HEIGHT - 1) * VGA_BUFFER_WIDTH + col]
                .write(Char { byte: b' ', color });
        }
        *ptr -= VGA_BUFFER_WIDTH;
    }
}

impl io::Write for Console {
    fn write_byte(&mut self, byte: u8) {
        self.buffer.write(&mut self.ptr, byte, self.color);
        self.update_cursor();
    }
    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.buffer.write(&mut self.ptr, *byte, self.color);
        }
        self.update_cursor();
    }
    fn write_string(&mut self, str: &str) {
        for byte in str.bytes() {
            match byte {
                0x20..=0x7E => self.buffer.write(&mut self.ptr, byte, self.color),
                b'\n' => self.newline_raw(),
                b'\t' => self.write_string("    "),
                _ => self.buffer.write(&mut self.ptr, b'?', self.color),
            }
        }
        self.update_cursor();
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Seek for Console {
    fn seek(&mut self, pos: usize) {
        self.ptr = pos;
        self.update_cursor()
    }

    #[inline]
    fn get_cursor_position(&self) -> usize {
        self.ptr
    }
}

impl Clear for Console {
    fn clear(&mut self) {
        self.seek_raw(0);
        for i in 0..VGA_BUFFER_SIZE {
            let mut copy = i;
            self.buffer.write(&mut copy, b' ', self.color);
        }
        self.seek(0);
    }
}

impl io::Read for Console {
    fn read_byte(&self) -> u8 {
        self.buffer.buffer[self.ptr].read().byte
    }
}

impl Console {
    #[inline]
    pub fn set_color(&mut self, color: ColorCode) {
        self.color = color;
    }
    #[inline]
    pub fn get_color(&self) -> ColorCode {
        self.color
    }

    pub fn update_cursor(&self) {
        unsafe {
            // 0x3D4/5 - the I/O ports for the VGA cursor
            // we send each byte of the cursor position in turn.
            crate::kernel::io::outb(0x3D4, 0x0E);
            crate::kernel::io::outb(0x3D5, ((self.ptr >> 8) & 0xFF) as u8);
            crate::kernel::io::outb(0x3D4, 0x0F);
            crate::kernel::io::outb(0x3D5, (self.ptr & 0xFF) as u8);
        }
    }

    /// Set the cursor's logical position without updating its visual position.
    #[inline]
    pub fn seek_raw(&mut self, value: usize) {
        self.ptr = value;
    }

    /// Move the cursor's logical position to the next line without updating its visual position.
    #[inline]
    pub fn newline_raw(&mut self) {
        // rounds the pointer up to the nearest multiple of VGA_BUFFER_WIDTH
        self.ptr = VGA_BUFFER_WIDTH * (self.ptr / VGA_BUFFER_WIDTH + 1);
    }

    pub fn newline(&mut self) {
        self.newline_raw();
        self.update_cursor()
    }

    pub fn backspace(&mut self) {
        // if we're at the first character, there's nothing more to erase.
        if self.ptr == 0 { return; }
        let mut new_pos = self.ptr - 1;
        self.buffer.write(&mut new_pos, b' ', self.color);
        // if we backspaced a newline, go to the end of the text in the previous line
        if new_pos % VGA_BUFFER_WIDTH == 0 {
            // don't overflow to the previous line if there's no text
            while self.ptr > new_pos - VGA_BUFFER_WIDTH &&
                self.buffer.buffer[self.ptr - 1].read() == (Char { byte: b' ', color: self.color })
            {
                self.ptr -= 1;
            }
        } else {
            self.ptr -= 1;
        }
        self.update_cursor();
    }
}

pub(crate) static CONSOLE: Lazy<Mutex<Console>> = Lazy::new(|| {
    Mutex::new(Console {
        ptr: 0,
        color: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xB8000 as *mut Buffer) }, // address of VGA text buffer
    })
});

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    let ints_enabled = crate::kernel::interrupts::is_enabled();
    if ints_enabled {
        // interrupts might mean another print, which would cause a deadlock.
        crate::kernel::interrupts::disable();
    }
    let result = {
        let mut console = CONSOLE.lock();
        console.write_fmt(args)
    };
    result.unwrap();
    if ints_enabled {
        crate::kernel::interrupts::enable();
    }
}
