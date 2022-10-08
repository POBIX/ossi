global load

section .boot

load:
  bits 16
  xor ax, ax
  mov ds, ax
  mov ss, ax

  mov bp, 0x9000 ; initialize the stack
  mov sp, bp

  ; load_kernel parameters:
  xor dh, dh
  ; dl has already been set to the boot disk by the BIOS
  mov cl, 2 ; boot sector + 1
  mov ax, KERNEL_SECTORS
  mov bx, KERNEL_ADDR
  call load_kernel

  jnc enter_protected

  .error:
    mov ebx, 0xB8000
    mov byte [ebx], ah
    jmp $

gdt:
  CODE_SEG equ .code - gdt
  DATA_SEG equ .data - gdt
  .null: ; mandatory null descriptor
    dd 0x0
    dd 0x0
  .code: ; code segment descriptor
    ; base=0x0, limit=0xFFFFF,
    ; 1st flags: (present)1 (privilege)00 (descriptor type)1 -> 1001b
    ; type flags: (code)1 (conforming)0 (readable)1 (accessed)0 -> 1010 b
    ; 2nd flags: (granularity)1 (32-bit default)1 (64-bit seg)0 (AVL)0 -> 1100 b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 10011010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  .data: ; data segment descriptor
    ; Same as code segment except for the type flags:
    ; type flags: (code)0 (expand down)0 (writable)1 (accessed)0 -> 0010 b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 10010010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  gdt_end: ; used to calculate size of GDT descriptor

  gdt_descriptor:
    dw gdt_end - gdt - 1 ; size
    dd gdt ; addr

protected_mode:
  enter_protected:
    bits 16
    cli ; disable interrupts during the switch
    lgdt [gdt_descriptor]

    mov eax, cr0 ; toggling the first bit of cr0 enters PM
    or eax, 1
    mov cr0, eax

    jmp CODE_SEG:init_protected

  init_protected:
    bits 32
    mov ax, DATA_SEG
    mov ds, ax
    mov ss, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov ebp, 0x90000 ; initialize the stack at the top of our new free space
    mov esp, ebp

    jmp _start ; rust function

%include "src/load_kernel.asm"

; calculated in linker.ld
extern KERNEL_SECTORS
extern KERNEL_ADDR

extern _start

times 510-($-$$) db 0 ; pad the binary to 510 bytes (+ the magic number)
dw 0xAA55 ; magic number that informs the BIOS that this is a boot sector
