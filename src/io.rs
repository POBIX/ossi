use core::arch::asm;

pub trait Write {
    fn write_byte(&mut self, byte: u8);
    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.write_byte(*byte);
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
    fn get_position(&self) -> usize;
}

pub trait Clear {
    fn clear(&mut self);
}

pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value
    );
}
