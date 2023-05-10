use crate::interrupts::GateType;
use crate::{interrupts, pic, println};
use alloc::boxed::Box;
use spin::Mutex;
use crate::events::{Event, EventHandler};
use core::arch::asm;

pub fn init() {
    unsafe {
        // attach on_tick to IRQ0
        interrupts::IDT[pic::IRQ_OFFSET + 0] =
            interrupts::Handler::new(on_tick, GateType::DInterrupt, 0);
    }
}

#[allow(named_asm_labels)]
extern "x86-interrupt" fn on_tick() {
    unsafe { TIMER += 1; }
    ON_TICK.lock().invoke(());

    // If we haven't yet initialised the heap, don't run the task scheduler code
    if !crate::heap::has_init() || !crate::process::has_loaded_processes() {
        pic::send_eoi(0);
        return;
    }

    let context = Box::new(crate::process::Context { esp: 0, eip: 0 });
    let context_ptr = Box::into_raw(context);
    unsafe {
        asm!(
            "mov [edi], esp", // undo push
            "lea eax, end_of_on_tick",
            "mov [edi+4], eax",
            in("edi") context_ptr,
        );
    }

    crate::process::next_program(context_ptr, || pic::send_eoi(0)); // jumps out of this function

    unsafe {
        asm!(
            ".global end_of_on_tick",
            "end_of_on_tick:"
        )
    }
}

#[inline]
pub fn get_ticks() -> u64 { unsafe { TIMER } }

pub static ON_TICK: Mutex<Event<()>> = Mutex::new(Event::<>::new());
static mut TIMER: u64 = 0;
