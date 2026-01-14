use alloc::{vec::Vec, boxed::Box};
use core::{arch::asm, ptr::NonNull};
use spin::Mutex;

/// The context of the CPU at the time of process switching.
/// These are the only registers we need since the interrupt handler saves the rest of the state based on the stack.
#[repr(C, packed)]
pub struct Context {
    pub esp: u32,
    pub eip: u32,
    pub dir: *mut crate::paging::PageDirectory
}

#[derive(Clone, Copy)]
pub(crate) struct Process {
    pub ctx: NonNull<Context>,
    pub data: crate::heap::ProcessHeapData
}

impl Process {
    pub fn new(ctx: *mut Context) -> Self {
        return Process {
            ctx: NonNull::new(ctx).unwrap(),
            data: crate::heap::ProcessHeapData::new()
        };
    }
}

unsafe impl Sync for Process {}
unsafe impl Send for Process {}

static PROCESSES: Mutex<Vec<Process>> = Mutex::new(Vec::new());
static CURR_INDEX: Mutex<usize> = Mutex::new(0);

fn next_index(curr_index: usize, proc_len: usize) -> usize {
    if curr_index < proc_len - 1 {
        curr_index + 1
    } else {
        0
    }
}

fn prev_index(curr_index: usize, proc_len: usize) -> usize {
    if curr_index > 0 {
        curr_index - 1
    } else {
        proc_len - 1
    }
}

/// Switches from the current program to the next one, while updating the context of the current one
pub(crate) fn next_program(new_context: *mut Context) {
    let new_process: Process;
    // Since we jump out of this function unbeknownst to the compiler (via the `ret`), we add an artificial scope.
    {
        let mut processes = PROCESSES.lock();
        if processes.len() == 0 { return; }
        let mut curr_index = CURR_INDEX.lock();

        // Assign our context to the previous index
        let replace = prev_index(*curr_index, processes.len());
        unsafe { drop(Box::from_raw(processes[replace].ctx.as_ptr())); } // free previous context
        processes[replace] = Process::new(new_context);

        new_process = processes[*curr_index];

        *curr_index = next_index(*curr_index, processes.len());
    }
    unsafe {
        (*new_process.ctx.as_mut().dir).switch_to();
        asm!(
            // Restore stack
            "mov esp, [edi]",
            // Restore eip
            "push [edi+4]", // Push eip
            "ret", // jmp to the pushed eip
            in("edi") new_process.ctx.as_ptr(),
            options(noreturn)
        );
    }
}

static mut HAS_LOADED_PROCESSES: bool = false;

pub(crate) fn register(esp: u32, eip: u32, dir: *mut crate::paging::PageDirectory) {
    // Since we lock PROCESSES, we can't switch programs while registering one.
    crate::pic::set_mask(0, true);

    let context = Box::new(Context { esp, eip, dir });
    let ptr = Box::into_raw(context);

    let len: usize;
    {
        let mut processes = PROCESSES.lock();
        processes.push(Process::new(ptr));
        len = processes.len();
        let mut curr_index = CURR_INDEX.lock();
        *curr_index = next_index(*curr_index , len);
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

pub(crate) fn unregister_prev() {
    // Since we lock PROCESSES, we can't switch programs while unregistering one.
    crate::pic::set_mask(0, true);

    let mut curr_index = CURR_INDEX.lock();
    let processes = PROCESSES.lock();
    *curr_index = prev_index(*curr_index, processes.len());
    unsafe { (*(processes[*curr_index].ctx.as_ptr())).eip = kill_process as u32; };

    crate::pic::set_mask(0, false);
}

fn kill_process() {
    let curr;
    {
        let mut processes = PROCESSES.lock();
        let len = processes.len();
        let curr_index = CURR_INDEX.lock();
        processes.remove(prev_index(*curr_index, len));
        curr = processes[*curr_index].ctx.as_ptr();
    }
    next_program(curr);
}

pub fn has_loaded_processes() -> bool { unsafe { HAS_LOADED_PROCESSES } }

pub fn get_curr_process() -> Process { PROCESSES.lock()[*CURR_INDEX.lock()] }
