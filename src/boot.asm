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

; allocate space for a stack
section .bss
align 16
stack_bottom:
  resb 16384
stack_top:

; protected mode entry point to the kernel (as defined in linker.ld)
section .text
global _start:function (_start.end - _start)
_start:
  mov esp, stack_top ; initialize stack
  extern main ; main is defined in rust
  call main

  ; once the kernel returns (this should theoretically never happen after it's done), loop forever
  cli
  .loop:
    hlt
    jmp .loop
  .end:

