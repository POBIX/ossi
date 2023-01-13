use core::arch::asm;

/// Runs the compiled code in program, starting at main_offset,
/// and returns the error code returned by the program in EAX.
pub unsafe fn run_program(main_offset: usize, program: &[u8]) -> u32 {
    let mut ret_val: u32;
    asm!(
        // do a FAR call to prog:offset. https://stackoverflow.com/a/52546754/13228993
        "push {prog}",
        "push {offset}",
        "mov ebp, esp",
        "lcall [ebp]",
        "add sp, 8",
        prog = in(reg) program.as_ptr(),
        offset = in(reg) main_offset,
        out("ax") ret_val
    );
    ret_val
}
