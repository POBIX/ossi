ENTRY(load)

SECTIONS {
    . = 0x7C00;
    .boot : {
        *(.boot)
    }

    KERNEL_ADDR = 0x8000;
    . = KERNEL_ADDR;
    .kernel : AT(0x7E00) {
        *(.text.*)
        *(.rodata.*)
        *(.data.*)
        _bss_start = .;
        *(.bss.*)
        *(COMMON)
        _bss_end = .;
        *(.debug_gdb_scripts)
    }

    KERNEL_SECTORS = (SIZEOF(.kernel) + 511) / 512;
}
