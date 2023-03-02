; i decided to use GRUB for the time being since i couldn't get the bootloader to work for kernels > 480KB
; (you'd have to switch back and forth between real and protected mode multiple times, and that's just too much effort.)
; (the kernel size jumps to 700KB as soon as any function from the rust std is used)
; but this is just temporary. after i figure out hardware drivers and interrupts and stuff, a custom bootloader will return.

; based off https://wiki.osdev.org/Bare_Bones#Booting_the_Operating_System

bits 32

; constants for multiboot header
MBALIGN equ 1 << 0 ; flag: align loaded modules on page boundaries
MEMINFO equ 1 << 1 ; flag: provide memory map
FLAGS equ MBALIGN | MEMINFO
MAGIC equ 0x1BADB002 ; magic number that lets bootloader find the header
CHECKSUM equ -(MAGIC + FLAGS)

; the multiboot header informs the bootloader that this is a kernel.
; the bootloader is going to search for this header, and it's in its own section
; so that we can force it to be inside the first 8KB of the kernel (the bootloader will only search for the header there)
section .multiboot
align 4
  dd MAGIC
  dd FLAGS
  dd CHECKSUM

gdt:
  CODE_SEG equ .code - gdt
  DATA_SEG equ .data - gdt
  USER_CODE_SEG equ .user_code - gdt
  USER_DATA_SEG equ .user_data - gdt
  GDT_ENTRIES_ADDR equ gdt
  .null: ; mandatory null descriptor
    dd 0x0
    dd 0x0
  .code: ; supervisor mode code segment descriptor
    ; base=0x0, limit=0xFFFFF,
    ; 1st flags: (present)1 (privilege)00 (descriptor type)1 -> 1001b
    ; type flags: (code)1 (conforming)0 (readable)1 (accessed)0 -> 1010b
    ; 2nd flags: (granularity)1 (32-bit default)1 (64-bit seg)0 (AVL)0 -> 1100b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 10011010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  .data: ; supervisor mode data segment descriptor
    ; Same as code segment except for the type flags:
    ; type flags: (code)0 (expand down)0 (writable)1 (accessed)0 -> 0010b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 10010010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  .user_code: ; usermode code segment descriptor
    ; base=0x0, limit=0xFFFFF,
    ; 1st flags: (present)1 (privilege)11=user (descriptor type)1 -> 1111b
    ; type flags: (code)1 (conforming)0 (readable)1 (accessed)0 -> 1010b
    ; 2nd flags: (granularity)1 (32-bit default)1 (64-bit seg)0 (AVL)0 -> 1100b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 11111010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  .user_data: ; uesrmode data segment descriptor
    ; Same as code segment except for the type flags:
    ; type flags: (code)0 (expand down)0 (writable)1 (accessed)0 -> 0010b
    dw 0xFFFF ; Limit (bits 0-15)
    dw 0x0 ; Base (bits 0-15)
    db 0x0 ; Base (bits 16-23)
    db 11110010b ; 1st flags, type flags
    db 11001111b ; 2nd flags, Limit (bits 16-19)
    db 0x0 ; Base (bits 24-31)
  gdt_end: ; used to calculate size of GDT descriptor

  gdt_descriptor:
    dw gdt_end - gdt - 1 ; size
    dd gdt ; addr

; allocate space for a stack
section .bss
align 16
stack_bottom:
  resb 16384
stack_top:


; protected mode entry point to the kernel (as defined in linker.ld)
section .text
global _start:function (new_cs.end - _start)
_start:
  cli

  lgdt [gdt_descriptor]
  jmp CODE_SEG:new_cs

new_cs:
  mov dx, DATA_SEG
  mov ds, dx
  mov es, dx
  mov fs, dx
  mov gs, dx
  mov ss, dx

  mov esp, stack_top ; initialize stack

  extern main ; main is defined in rust
  push eax ; magic number to verify successful GRUB boot
  push ebx ; address of a src/grub.rs::MultibootInfo struct, provided to us by GRUB.
  call main

  ; once the kernel returns (this should theoretically never happen after it's done), loop forever
  cli
  .loop:
    hlt
    jmp .loop
  .end:

global CODE_SEG
global DATA_SEG
global USER_CODE_SEG
global USER_DATA_SEG
global GDT_ENTRIES_ADDR
