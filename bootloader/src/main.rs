#![feature(rustc_private, lang_items)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


#[no_mangle]
extern fn entry() {
    serial::init();

    serial::write(b"Hello world!\n");

    cpu::halt();
}

