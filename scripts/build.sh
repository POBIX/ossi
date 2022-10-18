#!/usr/bin/env sh

OBJ_DIR="target/x86/debug"
LINK_ARGS="-T linker.ld $OBJ_DIR/boot.o -L$OBJ_DIR -lossi"

# compile

if ! cargo build ; then
  exit 1
elif ! nasm src/boot.asm -f elf32 -g -F dwarf -o $OBJ_DIR/boot.o ; then
  exit 1

# output debug symbols
elif ! i686-elf-ld $LINK_ARGS -o $OBJ_DIR/symbols_dump.tmp ; then
  exit 1
elif ! objcopy --only-keep-debug $OBJ_DIR/symbols_dump.tmp $OBJ_DIR/symbols.debug ; then
  exit 1
elif ! rm $OBJ_DIR/symbols_dump.tmp ; then
  exit 1

# link for real
elif ! i686-elf-gcc $LINK_ARGS -ffreestanding -nostdlib -lgcc -o iso/boot/ossi.bin ; then
  exit 1

# turn into ISO
elif ! grub2-mkrescue -o ossi.iso iso ; then
  exit 1
fi
exit 0
