ENTRY(_start)
SECTIONS {
    /* load sections at 100MB, where there's loads of available memory) */
    . = 100M;

    .text BLOCK(4K) : ALIGN(4K) {
        *(.multiboot) /* the multiboot section has to go first because it has to be in the first 8KB of the kernel */
        *(.text) /* the entirety of the code */
    }

    .rodata BLOCK(4K) : ALIGN(4K) {
        *(.rodata)
    }

    .data BLOCK(4K) : ALIGN(4K) {
        *(.data)
    }

    .bss BLOCK(4K) : ALIGN(4K) {
        *(COMMON)
        *(.bss)
    }
}
