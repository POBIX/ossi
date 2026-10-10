use core::arch::asm;

#[inline] pub(crate) unsafe fn outb(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value);
}
#[inline] pub(crate) unsafe fn outw(port: u16, value: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value);
}
#[inline] pub(crate) unsafe fn outl(port: u16, value: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") value);
}

#[inline] pub(crate) unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    asm!("in al, dx", in("dx") port, out("al") ret);
    ret
}
#[inline] pub(crate) unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    asm!("in ax, dx", in("dx") port, out("ax") ret);
    ret
}
#[inline] pub(crate) unsafe fn inl(port: u16) -> u32 {
    let ret: u32;
    asm!("in eax, dx", in("dx") port, out("eax") ret);
    ret
}

/// wait a very small amount of time by writing to an unused I/O port.
#[inline] pub(crate) fn wait() { unsafe { outb(0x80, 0); } }
