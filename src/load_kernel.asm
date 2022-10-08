; based on https://stackoverflow.com/a/34521208/13228993

bits 16
section .boot

MAX_ATTEMPTS equ 5 ; maximum number of read attempts per sector before error

; parameters:
;   dl - drive number
;   dh - head
;   cl - sector
;   ax - sector count
;   es:bx - buffer
; output:
;   cf=0 -> ch,dh,cl = chs of next sector
;   cf=1 -> ah = error code, ch,dh,cl = chs of error sector
load_kernel:
  push es
  push di

  push bp
  mov bp, sp

  ; local variables
  push ax
  %define sector_count [bp - 2]
  push cx
  %define max_sector [bp - 4]
  push dx
  %define max_head [bp - 6]
  push bx
  %define max_cylinder [bp - 8]

  ; BIOS command: get CHS limits
  push es
  mov ah, 0x8
  int 0x13
  pop es
  jc .return

  ; store the cylinder info
  mov bx, cx
  xchg bl, bh
  shr bh, 6
  xchg max_cylinder, bx

  movzx dx, dh ; dh->dl + 0->dhs
  xchg max_head, dx

  and cx, 0x3F ; sector info is 6-bits (two high bits belong to cylinder)
  xchg max_sector, cx

  .read_next:
    mov di, MAX_ATTEMPTS
  .do_read:
    ; BIOS command: read sector (al=2)
    mov ax, 0x0201 ; read 1 sector
    int 0x13
    jnc .success
    push ax ; save error code (in ah)

    ; BIOS command: reset disk system
    xor ah, ah
    int 0x13
    pop ax

    dec di ; di = attempts left
    jnz .do_read

    ; failed to read sector
    stc
    jmp .return

  .success:
    dec word sector_count
    jz .done
    ; move buffer 512 bytes up
    mov ax, es
    add ax, 512/16
    mov es, ax
    jmp .read_next

  .done:
    call inc_chs
    xor ah, ah

  .return:
    mov sp, bp
    pop bp
    pop di
    pop es
    ret

inc_chs:
  ; calculate the 6-bit sector number
  mov al, cl
  and al, 0x3F
  cmp al, max_sector
  jb .inc_sector

  cmp dh, max_head
  jb .inc_head

  ; calculate the 10-bit cylinder number
  mov ax, cx
  xchg al, ah
  shr ah, 6
  cmp ax, max_cylinder
  jb .inc_cylinder

  .wrap: ; that was the last sector.. come back to the first one
    mov cx, 1
    xor dh, dh
    ret

  .inc_cylinder:
    inc ax,
    ; split 10-bit cylinder number over cl and ch
    shl ah, 6
    xchg al, ah
    mov cx, ax
    mov dh, 0
    inc cl
    ret
  .inc_head:
    inc dh
    and cl, 0xC0
  .inc_sector:
    inc cl
    ret
