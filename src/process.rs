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

unsafe impl Sync for Process {}
unsafe impl Send for Process {}

