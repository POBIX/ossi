use alloc::{vec::Vec, boxed::Box};
use core::arch::asm;
use spin::Mutex;

/// The context of the CPU at the time of process switching.
/// These are the only registers we need since the interrupt handler saves the rest of the state based on the stack.
#[repr(C, packed)]
pub struct Context {
    pub esp: u32,
    pub eip: u32
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct ContextPtr(*const Context);

unsafe impl Sync for ContextPtr {}
unsafe impl Send for ContextPtr {}

static PROCESSES: Mutex<Vec<ContextPtr>> = Mutex::new(Vec::new());
static CURR_INDEX: Mutex<usize> = Mutex::new(0);

/// Should only ever be invoked via syscall by the task scheduler.
/// Switches from the current program to the next one, while updating the context of the current one
/// after: this function will get executed right before the jump to the next program
pub(crate) fn next_program(new_context: *const Context, after: fn()) {
    let new_process: ContextPtr;
    // Since we jump out of this function unbeknownst to the compiler (via the `ret`), we add an artificial scope.
    {
        let mut processes = PROCESSES.lock();
        if processes.len() == 0 { after(); return; }
        let mut curr_index = CURR_INDEX.lock();

        // Assign our context to the previous index
        let replace = if *curr_index != 0 { *curr_index - 1 } else { processes.len() - 1 };
        unsafe { Box::from_raw(processes[replace].0 as *mut Context) }; // free previous context
        processes[replace] = ContextPtr(new_context);

        new_process = processes[*curr_index];

        if *curr_index < processes.len() - 1 {
            *curr_index += 1;
        } else {
            *curr_index = 0;
        }

        after();
    }
    unsafe {
        asm!(
            // Restore stack
            "mov esp, [edi]",
            // Restore eip
            "push [edi+4]", // Push eip
            "ret", // jmp to the pushed eip
            in("edi") new_process.0
        );
    }
}

pub fn register(esp: u32, eip: u32) {
    let context = Box::new(Context { esp, eip });
    let ptr = Box::into_raw(context);
    PROCESSES.lock().push(ContextPtr(ptr));
    crate::interrupts::disable();
    next_program(ptr, crate::interrupts::enable);
}

pub fn has_loaded_processes() -> bool { PROCESSES.lock().len() != 0 }
