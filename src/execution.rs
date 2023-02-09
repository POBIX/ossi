use core::arch::asm;

/// Runs the compiled code in program, starting at main_offset,
/// and returns the error code returned by the program in EAX.
pub unsafe fn run_program(main_offset: usize, program: &[u8]) -> u32 {
    let mut ret_val: u32;
    asm!(
        "call {fn_ptr}",
        fn_ptr = in(reg) program.as_ptr().add(main_offset),
        out("eax") ret_val
    );
    ret_val
}
