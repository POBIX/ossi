use crate::interrupts::GateType;
use crate::{interrupts, pic};

pub fn init() {
    unsafe {
        // attach on_tick to IRQ0
        interrupts::IDT[pic::IRQ_OFFSET + 0] =
            interrupts::Handler::new(on_tick, GateType::DInterrupt);
    }
}

extern "x86-interrupt" fn on_tick() {
    pic::send_eoi(0);
}

