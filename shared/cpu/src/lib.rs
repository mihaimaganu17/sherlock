//! x86 CPU routines
#![no_std]
use core::arch::asm;

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
        in("edx") (val << 32) as u32,
        in("eax") (val & 0xffff_ffff) as u32,
    )
}

/// Set the GS base
#[inline]
pub unsafe fn set_gs_base(base: u64) {
    wrmsr(0xC000_0101, base);
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
