OBJ_DIR := target/x86/debug
BOOT_OBJ := $(OBJ_DIR)/boot.o

NASM_ARGS := -f elf32 -g -F dwarf
BOOT_FILE := src/boot.asm
LIB_FILE := $(OBJ_DIR)/libossi.a

LD := i686-elf-ld
GCC := i686-elf-gcc
LINK_ARGS := -T linker.ld -L$(OBJ_DIR) -lossi
BIN_LINK_ARGS := $(LINK_ARGS) -ffreestanding -nostdlib -lgcc
SYMBOLS := $(OBJ_DIR)/symbols.debug

TEMP_FILE := $(OBJ_DIR)/temp.temp

ISO_DIR := $(OBJ_DIR)/iso
BIN_OUTPUT := $(ISO_DIR)/boot/ossi.bin
ISO_OUTPUT := $(OBJ_DIR)/ossi.iso

QEMU = qemu-system-i386
QEMU_ARGS = -cdrom $(ISO_OUTPUT)

GRUB_CFG = grub.cfg

all: $(ISO_OUTPUT) $(SYMBOLS)

build: $(ISO_OUTPUT)

debug: all
	screen -d -m $(QEMU) $(QEMU_ARGS) -S -s

run: build
	$(QEMU) $(QEMU_ARGS)

clean:
	@rm -rf $(OBJ_DIR)


$(LIB_FILE): $(wildcard src/*.rs)
	@cargo build
$(BOOT_OBJ): $(LIB_FILE)
	@nasm $(BOOT_FILE) $(NASM_ARGS) -o $(BOOT_OBJ)

$(SYMBOLS): $(BOOT_OBJ)
	@$(LD) $(BOOT_OBJ) $(LINK_ARGS) -o $(TEMP_FILE)
	@objcopy --only-keep-debug $(TEMP_FILE) $(SYMBOLS)
	@rm $(TEMP_FILE)

$(BIN_OUTPUT): $(BOOT_OBJ) | $(ISO_DIR)
	@$(GCC) $(BOOT_OBJ) $(BIN_LINK_ARGS) -o $(BIN_OUTPUT)

$(ISO_OUTPUT): $(BIN_OUTPUT) | $(ISO_DIR) | grub.cfg
	@grub2-mkrescue $(ISO_DIR) -o $(ISO_OUTPUT)

$(ISO_DIR):
	@mkdir -p $(ISO_DIR)/boot/grub
	@cp $(GRUB_CFG) $(ISO_DIR)/boot/grub/grub.cfg
