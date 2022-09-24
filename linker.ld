ENTRY(load)
OUTPUT_FORMAT(binary)

SECTIONS {
    . = 0x7C00;
    .boot : {
        *(.boot)
    }

    . = 0x8000;
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
}
