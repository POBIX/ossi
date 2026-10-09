use alloc::vec::Vec;
use core::{arch::asm, sync::atomic::{Ordering, AtomicU32, AtomicBool}};
use spin::Mutex;

/// The context of the CPU at the time of process switching. These are the only registers we need
/// since the interrupt handler saves the rest of the state based on the stack.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Context {
    pub esp: u32,
    pub eip: u32,
    pub dir: *mut crate::paging::PageDirectory
}

#[derive(Clone, Copy)]
pub struct Process {
    pub id: u32,
    pub ctx: Context,
}

static ID_INCREMENTER: AtomicU32 = AtomicU32::new(0);
impl Process {
    pub fn new(ctx: Context) -> Self { Process {
        id: ID_INCREMENTER.fetch_add(1, Ordering::Relaxed),
        ctx
    }}
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
pub(crate) fn next_program(new_context: Context) {
    let new_process: Process;
    // We jump out of this function unbeknownst to the compiler (`ret`), so we add our own scope.
    {
        let mut processes = PROCESSES.lock();
        if processes.len() == 0 { return; }
        let mut curr_index = CURR_INDEX.lock();

        let l = processes.len();
        processes[prev_index(*curr_index, l)] = Process::new(new_context);

        new_process = processes[*curr_index];

        *curr_index = next_index(*curr_index, processes.len());
    }
    unsafe {
        (*new_process.ctx.dir).switch_to();
        asm!(
            "mov esp, {stack}",
            "push {ip}",
            "ret", // jmp to the pushed eip
            stack = in(reg) new_process.ctx.esp,
            ip = in(reg) new_process.ctx.eip,
            options(noreturn)
        );
    }
}

static HAS_LOADED_PROCESSES: AtomicBool = AtomicBool::new(false);

pub(crate) fn register(esp: u32, eip: u32, dir: *mut crate::paging::PageDirectory) {
    // Since we lock PROCESSES, we can't switch programs while registering one.
    crate::pic::set_mask(0, true);

    let context = Context { esp, eip, dir };

    let len: usize;
    {
        let mut processes = PROCESSES.lock();
        processes.push(Process::new(context));
        len = processes.len();
        let mut curr_index = CURR_INDEX.lock();
        *curr_index = next_index(*curr_index , len);
    }

    HAS_LOADED_PROCESSES.store(true, Ordering::Relaxed);

    // If this is the first program being run, we need to manually call next_program,
    // or else it will free the context we just created and never enter the program.
    if len == 1 {
        // We had to disable the timer, so user programs must manually unmask the timer at the start
        next_program(context);
    }
    // Otherwise the task scheduler will automatically call it.
    crate::pic::set_mask(0, false);
}

pub(crate) fn unregister_prev() {
    // Since we lock PROCESSES, we can't switch programs while unregistering one.
    crate::pic::set_mask(0, true);

    let mut curr_index = CURR_INDEX.lock();
    let mut processes = PROCESSES.lock();
    *curr_index = prev_index(*curr_index, processes.len());
    processes[*curr_index].ctx.eip = kill_process as u32;

    crate::pic::set_mask(0, false);
}

fn kill_process() {
    let curr;
    {
        let mut processes = PROCESSES.lock();
        let len = processes.len();
        let curr_index = CURR_INDEX.lock();
        processes.remove(prev_index(*curr_index, len));
        curr = processes[*curr_index].ctx;
    }
    next_program(curr);
}

pub fn has_loaded_processes() -> bool { HAS_LOADED_PROCESSES.load(Ordering::Relaxed) }

pub fn get_curr_process() -> Process { PROCESSES.lock()[*CURR_INDEX.lock()] }
