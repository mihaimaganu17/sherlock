//! x86 CPU routines
#![no_std]
use core::arch::asm;

/// Output `val` to I/O port `addr`
pub unsafe fn out8(addr: u16, val: u8) {
    asm!(
        r#"out dx, al"#,
        in("dx") addr,
        in("al") val,
    );
}

/// Read an 8-bit value from I/0 port `addr`
pub unsafe fn in8(addr: u16) -> u8 {
    let val: u8;
    asm!(
        r#"in al, dx"#,
        out("al") val,
        in("dx") addr,
    );

    val
}

/// Disable inrettupts and halt forever
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
