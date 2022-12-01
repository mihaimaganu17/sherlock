#![feature(panic_info_message, alloc_error_handler, lang_items)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate core_reqs;

mod realmode;
mod mm;
mod panic;
mod pxe;

use parse_pe::PeParser;

#[no_mangle]
extern fn entry() -> !{
    serial::init();
    mm::init();

    // Download the kernel
    let kernel = pxe::download("sherlock.kern").unwrap();

    // Parse the kernel PE
    let pe = PeParser::parse(&kernel).expect("Failed to parse PE");

    pe.sections(|vaddr, vsize, raw, _, _, _| {
        serial::print!("{:x?} {:x?}\n", vaddr as usize, vsize as usize);
        Some(())
    });

    cpu::halt();
}

