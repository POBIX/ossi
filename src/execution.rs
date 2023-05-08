use core::arch::asm;

use crate::paging::{PageFlags, self};

/// Runs the compiled code in program, starting at main_offset,
/// and returns the error code returned by the program in EAX.
#[allow(named_asm_labels)]
#[allow(unreachable_code)]
unsafe fn enter_loaded_program(entry_point: u32, new_esp: u32) {
    // push the previous stack onto the new one (for recovery after the program's execution)
    asm!(
        "mov [{new_esp}], esp",
        new_esp = in(reg) new_esp - 4 // - 4 since push subtracts 4 from esp
    );

    // Load start_of_program_execution into the task scheduler with our entry point -
    // as soon as it's time to start executing our program, it'll call it

    // First we push the entry point (start_of_program_execution's parameter) to our new stack
    asm!(
        "mov [{new_esp}], {entry_point}",
        new_esp = in(reg) new_esp - 8, // - 8 to account for the pushed stack and the new value
        entry_point = in(reg) entry_point
    );

    // Then we pretend that start_of_program_execution is the middle of an already running program,
    // meaning the task scheduler will call it and set esp to the new stack.
    crate::process::register(
        new_esp-8, // new_esp-8 to account for the pushed values
        &start_of_program_execution as *const _ as u32
    ); 
}

unsafe fn start_of_program_execution(entry_point: u32) {
    crate::userspace::enter();

    let ret_val: u32;
    asm!(
        "call {fn_ptr}",
        "pop esp",
        fn_ptr = in(reg) entry_point,
        out("eax") ret_val,
    );
}

#[repr(C)]
struct ElfHeader {
    ident: [u8; 16],
    file_type: u16,
    machine: u16,
    version: u32,
    entry: u32,
    prog_offset: u32,
    sect_offset: u32,
    flags: u32,
    header_size: u16,
    prog_entry_size: u16,
    prog_header_len: u16,
    sect_entry_size: u16,
    sect_header_len: u16,
    sect_str_idx: u16,
}

#[repr(C)]
struct ElfProgramHeader {
    prog_type: u32,
    offset: u32,
    virt_addr: u32,
    phys_addr: u32,
    file_size: u32,
    mem_size: u32,
    flags: u32,
    align: u32
}

// Just a random address sometime after the end of the heap and before the kernel (calculated by hand).
// TODO: work on an actual heap that isn't hardcoded so this won't be a thing.
static mut MEM_START: usize = 0x4_400_000;

pub unsafe fn run_program(program: &[u8]) {
    let header_bytes = &program[..core::mem::size_of::<ElfHeader>()];
    let header: &ElfHeader = unsafe { core::mem::transmute(header_bytes.as_ptr()) };

    // Verify magic number
    if header.ident[0] != 0x7F || header.ident[1] != b'E' || header.ident[2] != b'L' || header.ident[3] != b'F' {
        panic!("Not a valid ELF program");
    }

    // Extract the program header from the raw program bytes
    let p_start = header.prog_offset as usize;
    let p_end = header.prog_offset + (header.prog_header_len*header.prog_entry_size) as u32;
    let p_hdr_bytes = &program[p_start..p_end as usize];

    let p_header_arr: *const ElfProgramHeader = unsafe { core::mem::transmute(p_hdr_bytes.as_ptr()) };
    let p_header: &[ElfProgramHeader] = unsafe { core::slice::from_raw_parts(p_header_arr, header.prog_header_len as usize) };

    // Create a new page directory for this executable
    let dir = paging::PageDirectory::curr();
    // (*dir).switch_to();

    // Load each entry in the program header into memory
    for entry in p_header {
        if entry.prog_type != 1 {
            continue;
        }

        (*dir).map_addresses(
            MEM_START,
            MEM_START + entry.mem_size as usize, 
            entry.virt_addr as usize,
            PageFlags::RW | PageFlags::USER
        );

        MEM_START += (entry.mem_size as usize / 0x1000 + 1) * 0x1000;

        // Copy the program into memory
        core::ptr::copy::<u8>(
            program.as_ptr().byte_add(entry.offset as usize),
            entry.virt_addr as *mut u8,
            entry.file_size as usize
        );

        // If mem_size>file_size, ELF dictates we zero out whatever's left
        if entry.mem_size > entry.file_size {
            core::ptr::write_bytes(
                (entry.virt_addr + entry.file_size) as *mut u8,
                0,
                (entry.mem_size - entry.file_size) as usize
            );
        }
    }

    // Map the program's new stack
    const STACK_SIZE: usize = 4096;
    (*dir).map_addresses(MEM_START, MEM_START + STACK_SIZE, MEM_START, PageFlags::USER | PageFlags::RW);
    let stack_end = MEM_START + STACK_SIZE;
    for i in (MEM_START..stack_end).step_by(4) {
        let ptr = i as *mut u32;
        unsafe { *ptr = 0xDEADBEEF };
    }

    unsafe {
        let aligned_stack_top = (MEM_START + STACK_SIZE - 4) & !0xF;
        enter_loaded_program(header.entry, aligned_stack_top as u32);
    };
}
