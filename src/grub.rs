// this file is pretty much a rust adaptation of some stuff from https://www.gnu.org/software/grub/manual/multiboot/html_node/multiboot_002eh.html

#[repr(C)]
pub(crate) struct MultibootInfo {
    pub flags: u32,
    pub mem_lower: usize,
    pub mem_upper: usize,
    // there are a ton of other fields, but at least for now, we only need the top 3, so why bother?
    _padding: [u8; 120-3*4] // 120 is the actual size of the struct, 3 fields * 4 bytes each.
}

/// GRUB puts this number into EAX. If its contents are different, something has gone really wrong
const MAGIC_NUMBER: u32 = 0x2BADB002;
/// Bitmask for finding out whether the low/high memory info in MultibootInfo is valid
const MULTIBOOT_INFO_MEMORY: u32 = 0x00000001;

pub(crate) fn verify(magic: u32, flags: u32) -> Result<(), &'static str> {
    if magic != MAGIC_NUMBER {
        return Err("GRUB magic number incorrect.");
    }
    if flags & MULTIBOOT_INFO_MEMORY == 0 {
        return Err("Basic multiboot memory info invalid.");
    }

    Ok(())
}