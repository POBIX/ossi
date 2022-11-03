@echo off

set "OBJ_DIR=target\x86\debug"
set "LINK_ARGS=-T linker.ld %OBJ_DIR%\boot.o -L%OBJ_DIR% -lossi"

:: compile

cargo build
nasm src\boot.asm -f elf32 -g -F dwarf -o %OBJ_DIR%\boot.o
:: output debug symbols
::i686-elf-ld %LINK_ARGS% -o %OBJ_DIR%/symbols_dump.tmp &&^
::objcopy --only-keep-debug %OBJ_DIR%/symbols_dump.tmp %OBJ_DIR%/symbols.debug &&^
::del %OBJ_DIR%\symbols_dump.tmp &&^
:: link for real
i686-elf-gcc %LINK_ARGS% -ffreestanding -nostdlib -lgcc -o iso\boot\ossi.bin
:: turn into ISO
grub2-mkrescue -o ossi.iso iso
