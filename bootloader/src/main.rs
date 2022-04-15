#![feature(rustc_private)]
#![no_std]
#![no_main]

mod lld_undefined;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
fn entry() {
    panic!("APPLES")
}

