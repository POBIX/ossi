global load
bits 16
section .boot

extern _start
extern load_kernel

; calculated in linker.ld
extern LOADER_SECTORS
extern LOADER_ADDR

load: jmp 0:init ; ensure CS=0
init:
  xor ax, ax
  mov ds, ax
  mov ss, ax

  ; initialize the stack (our code starts at 0x7C00, and BIOS code ends at 0x500,
  ; so as the stack grows downward it shouldn't overwrite anything).
  mov sp, 0x7C00

  call load_loader

  xor dh, dh ; pass DL (boot drive, supplied by the BIOS)
  xor edi, edi
  mov di, dx    ; as parameter to load_kernel.
  call load_kernel ; defined in C
  add sp, 2 ; clear the stack from the parameter we just pushed
  cmp ax, 1 ; did the function return true?
  jne .error

  ; loaded kernel successfully! now execute it
  push _start
  call enter_protected

  .error:
    mov ebx, 0xB8000
    mov byte [ebx], '&'

; funny function. reads the C code that reads the rust code.
load_loader:
  mov ah, 02 ; BIOS command: read sectors
  mov al, LOADER_SECTORS ; number of sectors to read
  xor ch, ch ; cylinder
  xor dh, dh ; head
  mov cl, 2 ; sector to start reading from
  ; DL has already been set to the boot drive by the BIOS.
  ; ES:BX = memory buffer
  xor bx, bx
  mov es, bx
  mov bx, LOADER_ADDR
  int 13h ; execute command
  jc .error
  ret

  .error:
    mov ebx, 0xB8000 ; address of VGA text buffer
    mov byte [ebx], ah ; error code is in AH


%include "src/bootloader/protected_mode.asm"


times 510-($-$$) db 0 ; pad the binary to 510 bytes (+ the magic number)
dw 0xAA55 ; magic number that informs the BIOS that this is a boot sector
