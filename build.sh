#!/usr/bin/env sh

# compile
cargo build
nasm src/bootloader.asm -f elf64 -g -F dwarf -o bootloader.o

# output debug symbols
ld -T linker.ld bootloader.o -Ltarget/debug -lossi -o symbols_dump.tmp
objcopy --only-keep-debug symbols_dump.tmp symbols.debug
rm symbols_dump.tmp

# link for real
ld -T linker.ld bootloader.o -Ltarget/debug -lossi --oformat=binary -o ossi.bin
