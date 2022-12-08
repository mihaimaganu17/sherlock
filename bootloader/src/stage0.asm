; Define the address where the bootloader is expected to be loaded
; The first sector will be placed at RAM address 0000:7C000.
; https://wiki.osdev.org/MBR_(x86)
[org 0x7c00]
; Make sure the code is designed to run on 16-bit mode
[bits 16]

entry:
    ; Disable interrupts and clear direction flag
    cli
    cld

    ; Enable the A20 line so that all memory can be accessed.
    ; USing the FAST A20 option.
    ; https://wiki.osdev.org/A20_Line
    in al, 0x92
    or al, 2
    out 0x92, al

    ; Clear Data Segment
    xor ax, ax
    mov ds, ax

    ; Load the Global Descriptor Table
    lgdt [ds:pm_gdt]

    ; Enable protected mode
    ; Fetch the control Register 0
    mov eax, cr0
    ; set PE(Protection Enable) bit in CR0
    or eax, (1 << 0)
    ; set the CR0 back
    mov cr0, eax

    ; Perform a far jump to selector 0x0008 (offset into GDT, pointing at a
    ; 32bit PM code segment descriptor
    ; to load CS with a proper PM32 descriptor
    ; This is the Kernel Mode Code Segment
    jmp 0x0008:protected_mode_entry

[bits 32]

protected_mode_entry:
    mov ax , 0x10
    mov es, ax
    mov ds, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

.wait_for_stack:
    pause
    xor al, al
    lock xchg byte [stack_avail], al
    test al, al
    jz .wait_for_stack

    ; Set up a basic stack
    mov esp, 0x7c00

    push dword bootloader_end

    ; jump into Rust
    call entry_point

; 32-bit protected mode GDT
align 8
; Declare/Initialize data for the GDT base
; https://wiki.osdev.org/GDT_Tutorial
pm_gdt_base:
    ; Null Descriptor
    dq 0x0000000000000000
    ; Kernel Mode Code Segment
    dq 0x00CF9A000000FFFF
    ; Kernel Mode Data Segment
    dq 0x00CF92000000FFFF

pm_gdt:
    ; Declare a size of the GDT itself
    dw (pm_gdt - pm_gdt_base) - 1
    ; And a pointer to the GDT itself
    dd pm_gdt_base

; Fill sector with 0's
times 510-($-$$) db 0
; The last 2 bytes must be the Bootloader special signature
dw 0xaa55

; Do not move this, it must stay at 0x7e00. We release the early boot stack once we get into the
; the kernel and are using a new stack. We write directly to this location.
stack_avail: db 1

times (0x8000 - 0x7c00)-($-$$) db 0

[bits 16]
ap_entry:
    jmp 0x0000:entry

times (0x8100 - 0x7c00)-($-$$) db 0
incbin "build/sherlock.flat"

; A marker for the end of the bootloader
bootloader_end:
