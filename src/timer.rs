use crate::interrupts::GateType;
use crate::{interrupts, pic};
use spin::Mutex;
use crate::events::{Event, EventHandler};

pub fn init() {
    unsafe {
        // attach on_tick to IRQ0
        interrupts::IDT[pic::IRQ_OFFSET + 0] =
            interrupts::Handler::new(on_tick, GateType::DInterrupt);
    }
}

extern "x86-interrupt" fn on_tick() {
    unsafe { TIMER += 1; }
    ON_TICK.lock().invoke(());
    pic::send_eoi(0);
}

#[inline]
pub fn get_ticks() -> u64 { unsafe { TIMER } }

pub static ON_TICK: Mutex<Event<()>> = Mutex::new(Event::<>::new());
static mut TIMER: u64 = 0;
