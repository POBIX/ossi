ENTRY(_start)
SECTIONS {
    /* load sections at 100MB, where there's loads of available memory) */
    KERNEL_LOAD_ADDR = 100M;
    . = KERNEL_LOAD_ADDR;

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

    /* the address at the end of everything, after all the previous sections were loaded. */
    KERNEL_END_ADDR = . + 4K;
}
