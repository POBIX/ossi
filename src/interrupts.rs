#[derive(Copy, Clone)]
pub enum GateType {
    Task = 0x5,
    WInterrupt = 0x6, // 16-bit interrupt gate
    WTrap = 0x7,      // 16-bit trap gate
    DInterrupt = 0xE, // 32-bit interrupt gate
    DTrap = 0xF,      // 32-bit trap gate
}

/// Disables interrupts if they were enabled on creation; restores to previous state on drop
pub struct InterruptGuard {
    enabled: bool
}

impl InterruptGuard {
    pub fn new() -> Self {
        let enabled: bool = crate::syscall::AreInterruptsEnabled::call();
        if enabled {
            crate::syscall::DisableInterrupts::call();
        }
        Self { enabled }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.enabled {
            crate::syscall::EnableInterrupts::call();
        }
    }
}

