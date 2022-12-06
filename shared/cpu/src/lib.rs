//! x86 CPU routines
#![no_std]
use core::arch::asm;

const IA32_APIC_BASE: u32 = 0x1b;
const IA32_GS_BASE: u32 = 0xc000_0101;

/// Returns true is the current CPU is the BSP, otherwise returns false
#[inline]
pub fn is_bsp() -> bool {
    (unsafe { rdmsr(IA32_APIC_BASE) } & (1 << 8)) != 0
}

/// Output `val` to I/O port `addr`
#[inline]
pub unsafe fn out8(addr: u16, val: u8) {
    asm!(
        r#"out dx, al"#,
        in("dx") addr,
        in("al") val,
    );
}

/// Read an 8-bit value from I/0 port `addr`
#[inline]
pub unsafe fn in8(addr: u16) -> u8 {
    let val: u8;
    asm!(
        r#"in al, dx"#,
        out("al") val,
        in("dx") addr,
    );

    val
}

/// Invalidate a page table entry
#[inline]
pub unsafe fn invlpg(addr: usize) {
    asm!(
        r#"invlpg [{}]"#,
        in(reg) addr,
    );
}

/// Write an MSR
#[inline]
pub unsafe fn wrmsr(msr: u32, val: u64) {
    asm!(
        r#"
        wrmsr
        "#,
        in("ecx") msr,
        in("edx") (val >> 32) as u32,
        in("eax") val as u32,
    )
}

/// Read a MSR
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let val_lo: u32;
    let val_hi: u32;
    asm!("rdmsr", in("ecx") msr, out("edx") val_hi, out("eax") val_lo);

    (val_lo as u64 | ((val_hi as u64) << 32)) as u64
}

/// Set the GS base
#[inline]
pub unsafe fn set_gs_base(base: u64) {
    wrmsr(IA32_GS_BASE, base);
}

/// Disable inrettupts and halt forever
#[inline]
pub fn halt() -> ! {
    loop {
        unsafe {
            asm!(
            r#"
                cli
                hlt
            "#);
        }
    }
}

/// Canonicalize an address
#[inline]
pub fn canonicalize_address(addr: u64) -> u64 {
    // x86 address space is only 48-bits. You cannot have a 64-bit address. So intel requires that
    // the addresses are sign extended
    // The following addresses are contiguous. The first + 1 is equal to the second
    // 0x0000_7fff_ffff_ffff
    // Oxffff_8000_0000_0000
    //
    // If we are trying to access an address which have the first last 2 bytes different from
    // 0x0000 or 0xffff, we get a general protection fault
    //
    // #GP -> catastrophic exception 
    //          if you want to get the address that made the fault, you have to disassemble at
    //          the faulting @rip and decode address operand
    //
    // #PF -> page fault, page violation, you get a cr2, faulting address
    (((addr as i64) << 16) >> 16) as u64
}
