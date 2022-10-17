#include "load_kernel.h"

asm (".code16gcc\n");

bool read_sectors(uint8_t num_sectors, uint8_t buf_seg, uint8_t buf_off, uint64_t start_sec, uint8_t drive_number) {
    DAP dap = {
        sizeof(DAP), 0, 1,
        buf_off, buf_seg, start_sec
    };

    unsigned char *vga = (unsigned char *)0xB8000;

    for (uint8_t i = 0; i < num_sectors; i++) {
        vga[2*i] = '.'; // print '.' i characters from the start of the console. (0xB8000=address of VGA buffer)

        for (int j = 0; j < MAX_ATTEMPTS; j++) {
            register uint8_t drive_dl asm("dl") = drive_number;
            register DAP *dap_ptr  asm("si") = &dap;
            asm goto (
                // BIOS command: extended read sectors: AH=0x42, SI=DAP pointer, DL=drive
                "mov  $0x42, %%ah\n"
                "int  $0x13\n"
                "jnc  %l[next_loop]\n" // if we succeeded, break out of this loop
                // if we failed, reset the disk and try again (go to the next loop)
                "xor  %%ah, %%ah\n"
                "int  $0x13\n"
                :
                :
                : "dl", "si"
                : next_loop
            );
            continue; // this line gets called if carry flag was on (read failed)

            next_loop: // no choice but to use labels as we call this from assembly
            if (j == MAX_ATTEMPTS - 1) // if we've failed too many times, give up.
                return false;
        }

        dap.start_sec++;
        dap.buf_seg += 32; // one sector up
    }

    return true;
}

uint8_t min(uint8_t a, uint8_t b) { return a < b ? a : b; }

void copy_mem(uint8_t *src, uint8_t *dst, uint32_t length) {
    for (int i = 0; i < length; i++)
        dst[i] = src[i];
}

bool load_kernel(uint8_t drive_number) {
    uint8_t i = 0;
    while (i < KERNEL_SECTORS) {
        unsigned char *vga = (unsigned char *)0xB8000;
        *vga = 't';
        uint8_t secs = min(MAX_READ, KERNEL_SECTORS - i);
        if (!read_sectors(secs, 0, KERNEL_ADDR, 4 + i, drive_number))
            return false;

        copy_mem((uint8_t *) (KERNEL_ADDR + 512 * i), (uint8_t *) (0x100000 + 512 * i), secs * 512);

        i += secs;
    }
    return true;
}
