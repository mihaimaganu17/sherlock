; This file implements random trampolines between 16-bit, 32-bit, and 64-bit modes. We don't do
; this in inline assembly as we don't have the ability to change the bitness of the instructions
; dynamically
[bits 32]

struc register_state
    .eax: resd 1
    .ecx: resd 1
    .edx: resd 1
    .ebx: resd 1
    .esp: resd 1
    .ebp: resd 1
    .esi: resd 1
    .edi: resd 1
    .efl: resd 1

    .es: resw 1
    .ds: resw 1
    .fs: resw 1
    .gs: resw 1
    .ss: resw 1
endstruc

section .text

global _invoke_realmode
_invoke_realmode:
    pushad
    lgdt [rmgdt]

    ; Set all selectors to data segments
    ; 0x10 is a stand in for your data segment
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ; 0x0008 is a stand-in for your code segment
    ; This jump, which acts a code resembles,
    ; it is the only way that cause the CS register to change value(allowed by the CPU)
    ; We have to substract the PROGRAM_BASE, because it is already put in a segment
    jmp 0x0008:(.foop - PROGRAM_BASE)

[bits 16]
.foop:
    ; Disable protected mode
    mov eax, cr0
    and eax, ~1
    mov cr0, eax

    ; Clear out all segments, but they should be cleared in the GDT setup
    xor ax, ax
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; [Not getting it] Set up a fake iret to do a long jump to switch to new cs.
    pushfd                          ; eflags
    push dword (PROGRAM_BASE >> 4) ; cs
    push dword (.new_func - PROGRAM_BASE) ; eip
    iretd 

.new_func:
    ; Get the arguments passed to this function, jump above everything pushed by pushad
    movzx ebx, byte [esp + (4*0x9)] ; arg1, interrupt number
    ; Get the IVT offset of this interrupt number (INT #)
    shl ebx, 2
    ; Pointer to registers
    mov eax, dword [esp + (4*0xa)] ; arg2, pointer to registers

    ; Set up interrupt fake stack frame. This is what the real mode routine will pop off the stack
    ; during its iret. We have to make it as we did the interrupt ourselves
    ; This is the Return stack state
    mov ebp, (.retpoint - PROGRAM_BASE)
    pushfw
    push cs
    push bp

    ; Set up the call for the interrupt by loading the contents of the IVT based on the
    ; interrupt number specified
    ; This is the call stack state
    pushfw
    ; Pushing the selector, the segment
    push word [bx+2]
    push word [bx+0]

    ; Load the register state specified
    mov ecx, dword [eax + register_state.ecx]
    mov edx, dword [eax + register_state.edx]
    mov ebx, dword [eax + register_state.ebx]
    mov ebp, dword [eax + register_state.ebp]
    mov esi, dword [eax + register_state.esi]
    mov edi, dword [eax + register_state.edi]
    mov eax, dword [eax + register_state.eax]

    ; Perform a long jump to the interrupt entry point, simulating a software interrupt instruction
    ; Pretend we are doing an interrupt and we are doing a long jump
    iretw

.retpoint:
    ; Clear interrupts for sketchy bioses
    cli

    ; Save off all registers
    push eax
    push ecx
    push edx
    push ebx
    push ebp
    push esi
    push edi
    pushfd
    push es
    push ds
    push fs
    push gs
    push ss

    ; Get a pointer to the registers
    mov eax, dword [esp + (4*0xa) + (4*8) + (5*2)] ; arg2, pointer to registers

    ; Update the register state with the post-interrupt register state.
    pop word [eax + register_state.ss]
    pop word [eax + register_state.gs]
    pop word [eax + register_state.fs]
    pop word [eax + register_state.ds]
    pop word [eax + register_state.es]
    pop dword [eax + register_state.efl]
    pop dword [eax + register_state.edi]
    pop dword [eax + register_state.esi]
    pop dword [eax + register_state.ebp]
    pop dword [eax + register_state.ebx]
    pop dword [eax + register_state.edx]
    pop dword [eax + register_state.ecx]
    pop dword [eax + register_state.eax]

    ; Load data segment for ldgt
    mov ax, (PROGRAM_BASE >> 4)
    mov ds, ax

    ; Enable protected mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; Load 32-bit protected mode GDT
    mov eax, (pmgdt - PROGRAM_BASE)
    lgdt [eax]
    
    ; Set all segments to data segments
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax 

    ; Long jump back to protected mode
    pushfd ;eflags
    push dword 0x0008 ;cs
    push dword backout ;eip
    iretd

[bits 32]

global _pxecall
_pxecall:
    pushad
    lgdt [rmgdt]

    ; Set all selectors to data segments
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    jmp 0x0008:(.foop - PROGRAM_BASE)

[bits 16]
.foop:
    ; Disable protected mode
    mov eax, cr0
    and eax, ~1
    mov cr0, eax

    ; Clear all segments
    xor ax, ax
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Perform a long jump to real-mode
    pushfd                                ; eflags
    push dword (PROGRAM_BASE >> 4)        ; cs
    push dword (.new_func - PROGRAM_BASE) ; eip
    iretd

.new_func:

    ;    pub fn pxecall(seg: u16, off: u16, pxe_call: u16,
    ;                   param_seg: u16, param_off: u16);
    movzx eax, word [esp + (4*0x9)] ; arg1, seg
    movzx ebx, word [esp + (4*0xa)] ; arg2, offset
    movzx ecx, word [esp + (4*0xb)] ; arg3, pxe_call
    movzx edx, word [esp + (4*0xc)] ; arg4, param_seg
    movzx esi, word [esp + (4*0xd)] ; arg5, param_off

    ; Set up PXE call parameters (opcode, offset, seg)
    push dx
    push si
    push cx

    ; Set up our return address from the far call
    mov ebp, (.retpoint - PROGRAM_BASE)
    push cs
    push bp

    ; Set up a far call via iretw
    pushfw
    push ax
    push bx

    iretw

.retpoint:
    ; Hyper-V has been observed to set the interrupt flag in PXE routines. We
    ; clear it ASAP.
    cli

    ; Clean up the stack from the 3 word parameters we passed to PXE
    add sp, 6

    ; Load data segment for lgdt
    mov ax, (PROGRAM_BASE >> 4)
    mov ds, ax

    ; Enable protected mode
    mov eax, cr0
    or  eax, 1
    mov cr0, eax

    ; Load 32-bit protected mode GDT
    mov  eax, (pmgdt - PROGRAM_BASE)
    lgdt [eax]

    ; Set all segments to data segments
    mov ax, 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Jump back to protected mode
    pushfd             ; eflags
    push dword 0x0008  ; cs
    push dword backout ; eip
    iretd

[bits 32]
backout:
    popad
    ret

section .data

; -----------------------------------------------------------------------------------------------

; 16-bit real mode GDT
; Each line represents and entry and a descriptor
align 8
rmgdt_base:
    ; The first entry should always be the Null descriptor
    dq 0x0000000000000000

    ; 16-bit (Flags = 0) Read-Only(AccessByte = 0x9a) code segment, base = PROGRAM_BASE,
    ; limit 0x0000FFFF 
    dq 0x00009a000000ffff | (PROGRAM_BASE << 16)

    ; 16-bit (Flags = 0) Read-Write(AccessByte = 0x9c) data segment, base = 0, limit 0x0000FFFF
    dq 0x000092000000ffff

rmgdt:
    ; Size of the global descriptor table in bytes
    dw (rmgdt - rmgdt_base) - 1
    ; Base address of the descriptor table
    dd rmgdt_base
    
; ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

; 32-bit protected mode GDT

align 8
pmgdt_base:
    dq 0x0000000000000000 ; Null descriptor
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF

pmgdt:
    dw (pmgdt - pmgdt_base) - 1
    dd pmgdt_base
