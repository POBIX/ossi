use core::arch::asm;
use core::ops::Deref;
use spin::Lazy;

#[derive(Copy, Clone)]
pub enum GateType {
    Task = 0x5,
    WInterrupt = 0x6, // 16-bit interrupt gate
    WTrap = 0x7,      // 16-bit trap gate
    DInterrupt = 0xE, // 32-bit interrupt gate
    DTrap = 0xF,      // 32-bit trap gate
}

#[repr(C, packed)]
pub struct StackInterruptFrame {
    ip: u16,
    cs: u16,
    flags: u16,
    sp: u16,
    ss: u16,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Handler {
    ptr_low: u16, // bits 0..15 of function pointer
    selector: u16,
    _reserved: u8,
    attributes: u8, // present bit, DPL (irrelevant for hardware interrupts), gate type
    ptr_high: u16,  // bits 16..31 of function pointer
}

#[repr(C, packed)]
pub struct IDTR {
    pub(crate) limit: u16,
    pub(crate) base: u32,
}

pub unsafe fn init() {
    // actually load the IDTR and enable interrupts
    asm!("lidt [{}]",
         "sti",
         in(reg) IDTR.deref()
    )
}

type ISR = extern "x86-interrupt" fn(&StackInterruptFrame);

impl Handler {
    pub fn new(handler: ISR, gate_type: GateType) -> Handler {
        let addr = handler as *const () as u32;

        // the code segment should absolutely never change.
        static CS: Lazy<u16> = Lazy::new(|| cs());

        Handler {
            ptr_low: (addr & 0xFFFF) as u16,
            ptr_high: (addr >> 16) as u16,
            selector: *CS,
            attributes: 0x80 + gate_type as u8, // adding 0x80 sets the present bit to 1 and the DPL to 0.
            _reserved: 0,
        }
    }

    pub const fn null() -> Handler {
        Handler {
            ptr_low: 0,
            selector: 0,
            _reserved: 0,
            attributes: 0,
            ptr_high: 0,
        }
    }
}

fn cs() -> u16 {
    let cs: u16;
    unsafe { asm!("mov bx, cs", out("bx") cs) }
    cs
}

impl IDTR {
    pub unsafe fn new() -> IDTR {
        IDTR {
            base: IDT.as_ptr() as u32,
            limit: (IDT.len() * core::mem::size_of::<Handler>()) as u16 - 1,
        }
    }
}

pub static mut IDT: [Handler; 256] = [Handler::null(); 256];
static IDTR: Lazy<IDTR> = unsafe { Lazy::new(|| IDTR::new()) };
