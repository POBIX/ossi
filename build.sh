#!/usr/bin/env sh

cargo build
nasm src/bootloader.asm -f elf64 -o bootloader.o
#ld -Ttext 0x7C00 bootloader.o -Ltarget/debug -lossi --oformat=binary -o ossi.bin
ld -T linker.ld bootloader.o -Ltarget/debug -lossi -o ossi.bin
