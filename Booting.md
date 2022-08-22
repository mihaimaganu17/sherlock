# Booting options
Check: Normal Booting vs PXE vs UEFI
Check: Selectors

## Steps
1. Disable interrupts
    1. IVT -> Interrupt vector table
    2. IDT -> Interrut descriptor table
2. Clear direction flags
    1. Make sure the memory/pointer is always incremented
3. Set the A20 line, fast A20 line
    Causes the processor to not compute the memory accesses modulo 1MiB
4. Clear DS
5. Load a 32-bit GDT
6. Enable protected mode
7. Jump to the code selector in the GDT
8. Load all the other selectors


