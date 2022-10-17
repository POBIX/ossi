section .boot
bits 16

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
  ; parameter: bx=place to jump to after entering PM
  enter_protected:
    cli ; real mode interrupt handlers are invalid in PM
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

    mov ebp, 0x7C00 ; reinitialize the stack at the top of free space
    mov esp, ebp

    jmp bx
