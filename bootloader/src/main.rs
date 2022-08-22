#![feature(rustc_private)]
#![no_std]
#![no_main]

mod lld_undefined;

use core::panic::PanicInfo;
use core::arch::asm;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
fn entry() {
    unsafe {
        core::ptr::write(0xb8000 as *mut u16, 0x0f45);
        asm!(r#"
            cli
            hlt
        "#);
    }
}

