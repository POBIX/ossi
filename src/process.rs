use alloc::{vec::Vec, boxed::Box};
use core::arch::asm;
use spin::Mutex;

/// The context of the CPU at the time of process switching.
/// These are the only registers we need since the interrupt handler saves the rest of the state based on the stack.
#[repr(C, packed)]
pub struct Context {
    pub esp: u32,
    pub eip: u32,
    pub dir: *mut crate::paging::PageDirectory
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct ContextPtr(*const Context);

unsafe impl Sync for ContextPtr {}
unsafe impl Send for ContextPtr {}

static PROCESSES: Mutex<Vec<ContextPtr>> = Mutex::new(Vec::new());
static CURR_INDEX: Mutex<usize> = Mutex::new(0);

/// Switches from the current program to the next one, while updating the context of the current one
pub(crate) fn next_program(new_context: *const Context) {
    let new_process: ContextPtr;
    // Since we jump out of this function unbeknownst to the compiler (via the `ret`), we add an artificial scope.
    {
        let mut processes = PROCESSES.lock();
        if processes.len() == 0 { return; }
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
    }
    unsafe {
        (*(*new_process.0).dir).switch_to();
        asm!(
            // Restore stack
            "mov esp, [edi]",
            // Restore eip
            "push [edi+4]", // Push eip
            "ret", // jmp to the pushed eip
            in("edi") new_process.0,
            options(noreturn)
        );
    }
}

static mut HAS_LOADED_PROCESSES: bool = false;

pub fn register(esp: u32, eip: u32, dir: *mut crate::paging::PageDirectory) {
    // Since we lock PROCESSES, we can't switch programs while registering one.
    crate::pic::set_mask(0, true);

    let context = Box::new(Context { esp, eip, dir });
    let ptr = Box::into_raw(context);

    let len: usize;
    {
        let mut processes = PROCESSES.lock();
        processes.push(ContextPtr(ptr));
        len = processes.len();
    }

    unsafe { HAS_LOADED_PROCESSES = true; }

    // If this is the first program being run, we need to manually call next_program,
    // or else it will free the context we just created and never enter the program.
    if len == 1 {
        // Since we had to disable the timer, user programs must manually unmask the timer at their start.
        next_program(ptr);
    }
    // Otherwise the task scheduler will automatically call it.
    crate::pic::set_mask(0, false);
}

pub fn unregister(id: usize) {
    // Since we lock PROCESSES, we can't switch programs while unregistering one.
    crate::pic::set_mask(0, true);
    PROCESSES.lock().remove(id);
    crate::pic::set_mask(0, false);
}

pub fn has_loaded_processes() -> bool { unsafe { HAS_LOADED_PROCESSES } }
