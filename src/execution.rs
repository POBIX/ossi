use core::arch::asm;

use crate::paging::{PageFlags, self};

/// Runs the compiled code in program, starting at main_offset,
/// and returns the error code returned by the program in EAX.
unsafe fn enter_loaded_program(entry_point: u32, new_esp: usize) -> u32 {
    let mut ret_val: u32;
    asm!(
        "mov edx, esp",
        "mov esp, {new_esp}",
        "push edx",
        "call {fn_ptr}",
        "pop esp",
        fn_ptr = in(reg) entry_point,
        out("eax") ret_val,
        new_esp = in(reg) new_esp,
        out("edx") _, // clobber
    );
    ret_val
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

    // Just a random address sometime after the end of the heap and before the kernel (calculated by hand).
    // TODO: work on an actual heap that isn't hardcoded so this won't be a thing.
    let mut mem_start = 0x3000000;

    // Create a new page directory for this executable
    let dir = paging::PageDirectory::curr();
    // (*dir).switch_to();

    // Load each entry in the program header into memory
    for entry in p_header {
        if entry.prog_type != 1 {
            continue;
        }

        (*dir).map_addresses(
            mem_start,
            mem_start + entry.mem_size as usize, 
            entry.virt_addr as usize,
            PageFlags::RW | PageFlags::USER
        );

        mem_start += (entry.mem_size as usize / 0x1000 + 1) * 0x1000;

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
    (*dir).map_addresses(mem_start, mem_start + 0x1000, mem_start, PageFlags::USER | PageFlags::RW);
    let stack_end = mem_start + 0x1000;
    for i in (mem_start..stack_end).step_by(4) {
        let ptr = i as *mut u32;
        unsafe { *ptr = 0xDEADBEEF };
    }

    unsafe {
        crate::userspace::enter();
        let aligned_stack_top = (mem_start + 0x1000 - 4) & !0xF;
        enter_loaded_program(header.entry, aligned_stack_top);
    };
}
