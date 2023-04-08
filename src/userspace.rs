use core::{mem::size_of, arch::asm};
use crate::{DATA_SEG, KERNEL_STACK_TOP};

#[repr(C, packed)]
struct GdtPtr {
    limit: u16,
    base: u32
}

#[repr(C, packed)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

pub fn init() {
    unsafe {
        let gdt = GdtPtr {
            limit: 47, // size of a GDT entry * number of entries - 1
            base: &GDT_ENTRIES_ADDR as *const _ as u32,
        };
        write_tss();
        asm!("lgdt [{gdt_ptr}]", gdt_ptr = in(reg) &gdt);
        tss_flush();
    }
}

unsafe fn tss_flush() {
    asm!(
        "mov ax, 0x2B",
        "ltr ax",
    )
}

/// The Task State Segment allows us to go back into Ring 0 (kernel space) from Ring 3 (user space)
#[repr(C, packed)]
struct TssEntry {
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

static mut TSS_ENTRY: TssEntry = TssEntry {prev_tss: 0, esp0: 0, ss0: 0, esp1: 0, ss1: 0, esp2: 0, ss2: 0, cr3: 0, eip: 0, eflags: 0, eax: 0, ecx: 0, edx: 0, ebx: 0, esp: 0, ebp: 0, esi: 0, edi: 0, es: 0, cs: 0, ss: 0, ds: 0, fs: 0, gs: 0, ldt: 0, trap: 0, iomap_base: 0 };

unsafe fn gdt_set_gate(gdt_entries: *mut GdtEntry, num: usize, base: u32, limit: u32, access: u8, gran: u8) {
    (*gdt_entries.add(num)).base_low = (base & 0xFFFF) as u16;
    (*gdt_entries.add(num)).base_middle = ((base >> 16) & 0xFF) as u8;
    (*gdt_entries.add(num)).base_high = ((base >> 24) & 0xFF) as u8;

    (*gdt_entries.add(num)).limit_low = (limit & 0xFFFF) as u16;
    (*gdt_entries.add(num)).granularity = ((limit >> 16) & 0x0F) as u8;

    (*gdt_entries.add(num)).granularity |= gran & 0xF0;
    (*gdt_entries.add(num)).access = access;
}

unsafe fn write_tss() {
    let base = &TSS_ENTRY as *const _ as u32;
    let limit = size_of::<TssEntry>() as u32;

    gdt_set_gate(&GDT_ENTRIES_ADDR as *const u32 as *mut GdtEntry, 5, base, limit, 0xE9, 0);

    TSS_ENTRY.ss0 = &DATA_SEG as *const _ as u32;
    TSS_ENTRY.esp0 = &KERNEL_STACK_TOP as *const _ as u32;
}

extern "C" {
    static USER_CODE_SEG: u32;
    static GDT_ENTRIES_ADDR: u32;
}

/// Enters userspace
pub unsafe fn enter() {
    asm!(
        "mov ax, (4 * 8) | 3", // user data segment with RPL 3
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax", // sysexit sets SS
        "xor edx, edx", // not necessary; set to 0
        "mov eax, edi",  
        "mov ecx, 0x174", // MSR specifier: IA32_SYSENTER_CS
        "wrmsr", // set sysexit segments
        "lea edx, 5f", // to be loaded into EIP
        "mov ecx, esp", // to be loaded into ESP
        "sysexit",
        "5:",
        in("edi") &USER_CODE_SEG as *const _ as i32 - 16 // new CS=edi+16, new SS=edi+24
    );
}
