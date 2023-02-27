use core::{mem::size_of, arch::asm};

use bitfield::bitfield;

use crate::{CODE_SEG, interrupts};

pub fn init() {

}

/// The Task State Segment allows us to go back into Ring 0 (kernel space) from Ring 3 (user space)
#[repr(C, packed)]
#[no_warn(dead_code)]
pub struct TssEntry {
    pub prev_tss: u32, // The previous TSS - if we used hardware task switching this would form a linked list.
    pub esp0: u32, // The stack pointer to load when we change to kernel mode.
    pub ss0: u32, // The stack segment to load when we change to kernel mode.
    pub esp1: u32, // Unused...
    pub ss1: u32,
    pub esp2: u32,
    pub ss2: u32,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u32, // The value to load into ES when we change to kernel mode.
    pub cs: u32, // The value to load into CS when we change to kernel mode.
    pub ss: u32, // The value to load into SS when we change to kernel mode.
    pub ds: u32, // The value to load into DS when we change to kernel mode.
    pub fs: u32, // The value to load into FS when we change to kernel mode.
    pub gs: u32, // The value to load into GS when we change to kernel mode.
    pub ldt: u32, // Unused...
    pub trap: u16,
    pub iomap_base: u16,
}

static mut TSS_ENTRY: TssEntry = TssEntry {prev_tss: 0, esp: 0, ss: 0, _unused: [0; 23]};

unsafe fn write_tss() {
    let base = &TSS_ENTRY as *const _ as u32;
    let limit = base + size_of::<TssEntry>();

    gdt_set_gate(5, base, limit, 0xE9, 0);

    TSS_ENTRY.ss0 = 0x10;
    TSS_ENTRY.esp0 = 0;
    TSS_ENTRY.cs = &CODE_SEG as *const _ as u32;

}

extern "C" {
    static USER_CODE_SEG: u32;
    static USER_DATA_SEG: u32;
}

/// Enters userspace
pub unsafe fn enter() {
    interrupts::disable(); // This is critical code. We can't risk interrupts changing something.
    asm!(
        // switch to the new data segment
        "mov ax, {ds}",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        // switch to the new stack
        "mov eax, esp",
        "push {ds}",
        "push eax",
        // push the flags. we enable interrupts through the or statement
        "pushf",
        "pop ax",
        "or ax, 0x200",
        "push ax",
        // push the address of the instruction to return to after switching mode
        "push {cs}",
        "push 5f",
        "iret",
        "5:",
        ds = in(reg) (&USER_DATA_SEG as *const _ as u32) | 0x3, // the or enters ring 3
        cs = in(reg) (&USER_CODE_SEG as *const _ as u32) | 0x3,
    );
}
