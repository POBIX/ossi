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
#[derive(Copy, Clone)]
pub struct Handler {
    ptr_low: u16, // bits 0..15 of function pointer
    selector: u16,
    _reserved: u8,
    attributes: u8, // present bit, DPL (irrelevant for hardware interrupts), gate type
    ptr_high: u16,  // bits 16..31 of function pointer
}

#[repr(C, packed)]
struct IDTR {
    pub(crate) limit: u16,
    pub(crate) base: u32,
}

pub fn init() {
    unsafe {
        setup_idt();
        // actually load the IDTR and enable interrupts
        asm!("lidt [{}]",
            "sti",
            in(reg) IDTR.deref()
        )
    }
}

type ISR = extern "x86-interrupt" fn();
type ISRErr = extern "x86-interrupt" fn(u32);

impl Handler {
    pub fn new_raw(addr: u32, gate_type: GateType, dpl: u8) -> Handler {
        Handler {
            ptr_low: (addr & 0xFFFF) as u16,
            ptr_high: (addr >> 16) as u16,
            selector: cs(),
            attributes: 0x80 | (dpl << 5) + gate_type as u8, // adding 0x80 sets the present bit to 1.
            _reserved: 0,
        }
    }

    pub fn new(isr: ISR, gate_type: GateType, dpl: u8) -> Handler {
        Handler::new_raw(isr as *const () as u32, gate_type, dpl)
    }

    pub fn new_err(isr: ISRErr, gate_type: GateType, dpl: u8) -> Handler {
        Handler::new_raw(isr as *const () as u32, gate_type, dpl)
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
static mut ENABLED: bool = false;

pub fn enable() {
    unsafe {
        asm!("sti");
        ENABLED = true;
    }
}

pub fn disable() {
    unsafe {
        asm!("cli");
        ENABLED = false;
    }
}

pub fn is_enabled() -> bool { unsafe { ENABLED } }

macro_rules! int_fn {
    ($name:tt) => {
        extern "x86-interrupt" fn $name() {
            panic!(concat!("EXCEPTION: ", stringify!($name)));
        }
    };
}

macro_rules! int_fn_err {
    ($name:tt) => {
        extern "x86-interrupt" fn $name(err: u32) {
            panic!(
                concat!("EXCEPTION: ", stringify!($name), ", ERROR CODE: {:#X}"),
                err
            );
        }
    };
}

macro_rules! int_set {
    ($idx:literal, $name:tt) => {
        IDT[$idx] = Handler::new($name, GateType::DInterrupt, 0);
    };
}

macro_rules! int_set_err {
    ($idx:literal, $name:tt) => {
        IDT[$idx] = Handler::new_err($name, GateType::DInterrupt, 0);
    };
}

// https://wiki.osdev.org/Exceptions
int_fn!(divide_by_zero);
int_fn!(debug);
int_fn!(non_maskable_interrupt);
int_fn!(breakpoint);
int_fn!(overflow);
int_fn!(bound_range_exceeded);
int_fn!(invalid_opcode);
int_fn!(device_not_available);
int_fn_err!(double_fault);
int_fn_err!(invalid_tss);
int_fn_err!(segment_not_present);
int_fn_err!(stack_segment_fault);
int_fn_err!(general_protection_fault);
int_fn_err!(page_fault);
int_fn!(x87_floating_point_exception);
int_fn_err!(alignment_check);
int_fn!(machine_check);
int_fn!(simd_floating_point_exception);
int_fn!(virtualization_exception);
int_fn_err!(control_protection_exception);
int_fn!(hypvervisor_injection_exception);
int_fn_err!(vmm_communication_exception);
int_fn_err!(security_exception);

unsafe fn setup_idt() {
    int_set!(0x0, divide_by_zero);
    int_set!(0x1, debug);
    int_set!(0x2, non_maskable_interrupt);
    int_set!(0x3, breakpoint);
    int_set!(0x4, overflow);
    int_set!(0x5, bound_range_exceeded);
    int_set!(0x6, invalid_opcode);
    int_set!(0x7, device_not_available);
    int_set_err!(0x8, double_fault);
    int_set_err!(0xA, invalid_tss);
    int_set_err!(0xB, segment_not_present);
    int_set_err!(0xC, stack_segment_fault);
    int_set_err!(0xD, general_protection_fault);
    int_set_err!(0xE, page_fault);
    int_set!(0x10, x87_floating_point_exception);
    int_set_err!(0x11, alignment_check);
    int_set!(0x12, machine_check);
    int_set!(0x13, simd_floating_point_exception);
    int_set!(0x14, virtualization_exception);
    int_set_err!(0x15, control_protection_exception);
    int_set!(0x1C, hypvervisor_injection_exception);
    int_set_err!(0x1D, vmm_communication_exception);
    int_set_err!(0x1E, security_exception);
}
