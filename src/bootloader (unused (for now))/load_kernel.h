#include "defs.h"

#define MAX_ATTEMPTS 5
#define LOADER __attribute__((section(".loader")))

// defined in linker.ld
extern uint8_t KERNEL_ADDR;
extern uint8_t KERNEL_SECTORS;
extern uint8_t MAX_READ;

typedef struct DAP {
    uint8_t size; // = sizeof(DAP)
    uint8_t reserved; // = 0
    uint16_t batch_size; // number of sectors to read at once. = 1 to avoid hitting the 64K boundary.
    uint16_t buf_off;
    uint16_t buf_seg;
    uint64_t start_sec;
} DAP;

bool read_sectors(uint8_t num_sectors, uint8_t buf_seg, uint8_t buf_off, uint64_t start_sec, uint8_t drive_number) LOADER;
uint8_t min(uint8_t a, uint8_t b) LOADER;
void copy_mem(uint8_t *src, uint8_t *dst, uint32_t length) LOADER;
bool load_kernel(uint8_t drive_number) LOADER;
