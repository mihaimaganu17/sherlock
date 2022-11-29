#![feature(rustc_private, panic_info_message, alloc_error_handler, lang_items)]
#![no_std]
#![no_main]

extern crate alloc;

mod lld_undefined;
mod realmode;
mod mm;
mod panic;

use serial::print;

#[no_mangle]
pub extern "C" fn __CxxFrameHandler3() {}

#[no_mangle]
extern fn entry() -> !{
    serial::init();
    mm::init();

    let mut map = alloc::collections::BTreeMap::new();
    map.insert(5u8, 50);
    map.insert(8u8, 50000);
    map.insert(39u8, 5);

    print!("{:?}\n", map);

    cpu::halt();
}

