bits 16
section .boot

; parameters:
;  number of sectors to read
;  segment of memory buffer
;  offset of memory buffer
;  first sector to read
;  DL=disk
; returns:
;  CF=0 -> read successfully
;  CF=1 -> AH=error code
read_sectors:
  push bp
  mov bp, sp
  %define num_sectors [bp+4]
  %define seg [bp+6]
  %define offset [bp+8]
  %define start_sec [bp+10]

  mov si, offset
  mov [dap_off], si
  mov si, seg
  mov [dap_seg], si
  mov si, start_sec
  mov [dap_start_sec], si

  .loop:
    mov di, 5 ; max number of read attempts
    mov ebx, 0xB8000
    add bx, word [print_counter]
    add word [print_counter], 2
    mov byte [ebx], '.'
    .do_read:
      ; BIOS command: extended read sectors: AH=0x42, DS:SI=DAP pointer, DL=drive
      mov ah, 0x42
      mov si, dap
      int 0x13
      jnc .next_loop

      ; if we failed, reset the disk and try again a few times
      push ax ; save error code (stored in AH)
      ; BIOS command: reset disk: AH=0x0, DL=drive
      xor ah, ah
      int 0x13
      pop ax

      dec di ; number of attempts left
      jnz .do_read

      ; reading the disk failed too many times, give up. return with the carry flag set and the error code in AH
      stc
      jmp .return

    .next_loop:
      ; should actually be qword (unsupported in real mode), so this will break if the kernel gets larger than 32 MB.
      inc dword [dap_start_sec]
      add word [dap_seg], 512/0x10 ; one sector up
      dec word num_sectors
      jnz .loop

  .return:
    pop bp
    ret 2*4


print_counter: dw 0

dap:
  db 0x10 ; size of DAP
  db 0 ; unused
  dw 1 ; number of sectors to read at a time
  dap_off: dw 0 ; offset of memory buffer
  dap_seg: dw 0 ; segment of memory buffer
  dap_start_sec: dq 1 ; first sector to read (LBA)
