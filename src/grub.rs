use core::arch::asm;

#[repr(C)]
pub(crate) struct MultibootInfo {
    drives_length: u32,
    drives_addr: u32,
    config_table: u32,
    boot_loader_name: u32,
    apm_table: u32,
    vbe_control_info: u32,
    vbe_mode_info: u32,
    vbe_mode: u32,
    vbe_interface_seg: u16,
    vbe_interface_off: u16,
    vbe_interface_len: u16,
    framebuffer_addr: u64,
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
    framebuffer_type: u8,
    union: FramebufferUnion
}

#[repr(C)]
pub(crate) struct FramebufferUnionA {
    framebuffer_palette_addr: u32,
    framebuffer_palette_num_colors: u16
}

#[repr(C)]
pub(crate) struct FramebufferUnionB {
    framebuffer_red_field_position: u8,
    framebuffer_red_mask_size: u8,
    framebuffer_green_field_position: u8,
    framebuffer_green_mask_size: u8,
    framebuffer_blue_field_position: u8,
    framebuffer_blue_mask_size: u8
}

impl core::ops::Drop for FramebufferUnionA {
    fn drop(&mut self) {}
}

impl core::ops::Drop for FramebufferUnionB {
    fn drop(&mut self) {}
}

#[repr(C)]
pub(crate) union FramebufferUnion {
    a: FramebufferUnionA,
    b: FramebufferUnionB
}

impl MultibootInfo {
    /// returns the MultibootInfo pointer that GRUB loads into EBX at startup.
    pub unsafe fn from_ebx() -> &'static MultibootInfo {
        let ret: u32;
        asm!("mov {:e}, ebx", out(reg) ret);
        ret as &MultibootInfo
    }
}
