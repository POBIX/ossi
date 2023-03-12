use core::arch::asm;

use crate::interrupts;

static SYSCALLS: [fn(); 3] = [||{}; 3];

fn init() {
    unsafe {
        interrupts::IDT[0x80] = interrupts::Handler::new(syscall_handler, interrupts::GateType::DInterrupt);
    }
}

extern "x86-interrupt" fn syscall_handler() {
    #[repr(C)]
    struct Registers {
        pub ds: u32,
        pub edi: u32,
        pub esi: u32,
        pub ebp: u32,
        pub esp: u32,
        pub ebx: u32,
        pub edx: u32,
        pub ecx: u32,
        pub eax: u32,
        pub int_no: u32,
        pub err_code: u32,
        pub eip: u32,
        pub cs: u32,
        pub eflags: u32,
        pub useresp: u32,
        pub ss: u32,
    }
    let regs: *mut Registers;
    unsafe {
        asm!("mov {}, [esp+4]", out(reg) regs);

        if (*regs).eax >= SYSCALLS.len() as u32 {
            panic!("Unrecognised syscall");
        }

        let location = SYSCALLS[(*regs).eax as usize];
        
        let ret: u32;
        asm!(
            "push {edi}",
            "push {esi}",
            "push {edx}",
            "push {ecx}",
            "push {ebx}",
            "call *{location}",
            "pop ebx", // we don't really need the values we popped,
            "pop ebx", // we only pop to clear the stack -
            "pop ebx", // we can't know how many arguments a syscall takes
            "pop ebx",
            "pop ebx",
            edi = in(reg) (*regs).edi,
            esi = in(reg) (*regs).esi,
            edx = in(reg) (*regs).edx,
            ecx = in(reg) (*regs).ecx,
            ebx = in(reg) (*regs).ebx,
            location = in(reg) location,
            lateout("eax") ret,
        );
        (*regs).eax = ret;
    }
}
