use bitfield::bitfield;

bitfield! {
    pub struct GdtEntryBits(u64);
    impl Debug;

    limit_low, set_limit_low: 15, 0;
    base_low, set_base_low: 39, 16;
    accessed, set_accessed: 40;
    read_write, set_read_write: 41;
    conforming_expand_down, set_conforming_expand_down: 42;
    code, set_code: 43;
    code_data_segment, set_code_data_segment: 44;
    dpl, set_dpl: 46, 45;
    present, set_present: 47;
    limit_high, set_limit_high: 51, 48;
    available, set_available: 52;
    long_mode, set_long_mode: 53;
    big, set_big: 54;
    gran, set_gran: 55;
    base_high, set_base_high: 63, 56;
}

pub fn init() {
    // In order to go into userspace, we first need to load new segments into the GDT.
    let mut ring3_code_seg = GdtEntryBits(0);

    // Note that since we initialise the bits to 0, we don't need to set the bits that should be 0.
    ring3_code_seg.set_limit_low(0xFFFF); // maximum possible address space
    ring3_code_seg.set_limit_high(0xF);
    ring3_code_seg.set_read_write(true); // since this is a CS, it should be readable.
    ring3_code_seg.set_code(true);
    ring3_code_seg.set_code_data_segment(true);
    ring3_code_seg.set_dpl(3); // ring 3
    ring3_code_seg.set_present(true);
    ring3_code_seg.set_available(true);
    ring3_code_seg.set_big(true); // 32-bit
    ring3_code_seg.set_gran(true); // 4KB page addressing
    let mut ring3_data_seg = ring3_code_seg; // the bits should be equal
    ring3_data_seg.set_code(false);
}
