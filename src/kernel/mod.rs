pub mod ata;
pub mod execution;
pub mod fs;
pub mod grub;
pub mod heap;
pub mod interrupts;
pub mod io;
pub mod keyboard;
pub mod paging;
pub mod pic;
pub mod process;
pub mod syscall;
pub mod timer;
pub mod userspace;
pub mod vga_console;

use crate::io::Clear;
use crate::kernel::vga_console::CONSOLE;

extern "C" {
    static CODE_SEG: usize;
    static DATA_SEG: usize;
    static KERNEL_STACK_TOP: usize;
}

#[no_mangle]
pub(crate) extern "C" fn kernel_main(info: &grub::MultibootInfo, magic: u32) -> ! {
    grub::verify(magic, info.flags).unwrap();

    pic::remap();
    timer::init();
    interrupts::init();


    userspace::init();
    syscall::init();

    paging::init();

    CONSOLE.lock().clear();
    keyboard::init();
    ata::init();

    execution::execute_file(&mut crate::fs::File::open("/shell").unwrap());

    loop {}
}
