use core::{mem::size_of, arch::asm};

use bitfield::bitfield;

use crate::interrupts;

bitfield! {
    pub struct GdtEntryBits(u64);
    impl Debug;

    u16, limit_low, set_limit_low: 15, 0;
    u32, base_low, set_base_low: 39, 16;
    accessed, set_accessed: 40;
    read_write, set_read_write: 41;
    conforming_expand_down, set_conforming_expand_down: 42;
    code, set_code: 43;
    code_data_segment, set_code_data_segment: 44;
    u8, dpl, set_dpl: 46, 45;
    present, set_present: 47;
    u8, limit_high, set_limit_high: 51, 48;
    available, set_available: 52;
    long_mode, set_long_mode: 53;
    big, set_big: 54;
    gran, set_gran: 55;
    u8, base_high, set_base_high: 63, 56;
}

pub fn init() {

}

/// The Task State Segment allows us to go back into Ring 0 (kernel space) from Ring 3 (user space)
#[repr(C, packed)]
struct TssEntry {
    prev_tss: u32,
    esp: u32,
    ss: u32,
    _unused: [u32; 23] // needed for the size to be correct
}

static TSS_ENTRY: TssEntry = TssEntry {prev_tss: 0, esp: 0, ss: 0, _unused: [0; 23]};

fn write_tss() -> GdtEntryBits {
    let base = &TSS_ENTRY as *const _ as u32;
    let limit = size_of::<TssEntry>();

    // Add a tss descriptor to the GDT
    let mut g = GdtEntryBits(0);
    g.set_limit_low(limit as u16);
    g.set_base_low(base);
    g.set_accessed(true); // For a system entry, true=TSS, false=LDT
    g.set_read_write(false); // For a TSS, indicates busy/not.
    g.set_code(true); // For a TSS, indicates 32/16bit.
    g.set_code_data_segment(false); // Same as accessed
    g.set_dpl(0); // Ring 0
    g.set_present(true);
    g.set_limit_high((limit & 0xF) as u8);
    g.set_base_high((base & 0xFF) as u8);

    let esp: u32;
    let ss: u32;

    unsafe {asm!(
        "mov {esp_tmp}, esp",
        "mov {ss_tmp}, ss",
        esp_tmp = out(reg) esp,
        ss_tmp = out(reg) ss
    );}

    

    g
}

extern "C" {
    static USER_CODE_SEG_ADDR: u32;
    static USER_DATA_SEG_ADDR: u32;
}
const USER_CODE_SEG: u32 = &USER_CODE_SEG_ADDR as *const _ as u32;

pub unsafe fn enter() {
    interrupts::disable(); // this is critical code. We can't risk interrupts changing something.
    asm!(
        "mov ax, {ds}",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        "mov eax, esp",
        "push {ds}",
        "push eax",
        "pushf",
        "push {cs}",
        "push 5f",
        "pop ax",
        "or ax, 0x200",
        "push ax",
        "iret",
        "5:",
        ds = const (&USER_CODE_SEG as *const _ as u32),
        cs = const USER_CODE_SEG | 0x3

    );
}
