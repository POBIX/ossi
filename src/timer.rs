use crate::interrupts::GateType;
use crate::{interrupts, pic};
use alloc::boxed::Box;
use spin::Mutex;
use crate::events::{Event, EventHandler};
use core::arch::asm;
use crate::paging::PageDirectory;

pub fn init() {
    unsafe {
        // attach on_tick to IRQ0
        interrupts::IDT[pic::IRQ_OFFSET + 0] =
            interrupts::Handler::new_raw(on_tick as u32, GateType::DInterrupt, 0);
    }
}

#[naked]
extern "x86-interrupt" fn on_tick() {
    unsafe {
        asm!(
            "pushfd",
            "pushad",
            "sub esp, 1024",
            "call on_tick_internal",
            "add esp, 1024",
            "popad",
            "popfd",
            "iretd",
            options(noreturn)
        );
    }
}

#[allow(named_asm_labels)]
#[no_mangle]
fn on_tick_internal() {
    // Force preservation of state.
    // Even though the "x86-interrupt" ABI preserves all used registers, some registers are not actually used
    // by the function. So they won't be preserved. Which is bad.
    // This causes some registers to be pushed twice, but there doesn't seem to be a simple fix.

    crate::interrupts::disable();
    unsafe { TIMER += 1; }
    ON_TICK.lock().invoke(());

    // If we haven't yet initialised the heap, don't run the task scheduler code
    if !crate::heap::has_init() || !crate::process::has_loaded_processes() {
        pic::send_eoi(0);
        crate::interrupts::enable();
        return;
    }

    let context = Box::new(crate::process::Context { esp: 0, eip: 0, dir: core::ptr::null_mut() });
    let context_ptr = Box::into_raw(context);
    unsafe {
        asm!(
            "mov [ebx], esp", // undo push
            "lea eax, end_of_on_tick",
            "mov [ebx+4], eax",
            "mov [ebx+8], ecx",
            in("ebx") context_ptr,
            in("ecx") PageDirectory::curr(),
            out("eax") _ // clobber
        );
    }

    crate::process::next_program(context_ptr); // jumps out of this function

    unsafe {
        asm!(
            ".global end_of_on_tick",
            "end_of_on_tick:",
        );
        pic::set_mask(0, false);
        pic::send_eoi(0);
        crate::interrupts::enable();
    }
}

#[inline]
pub fn get_ticks() -> u64 { unsafe { TIMER } }

pub static ON_TICK: Mutex<Event<()>> = Mutex::new(Event::<>::new());
static mut TIMER: u64 = 0;
