#![feature(panic_info_message)]
#![no_std]
#![no_main]

extern crate core_reqs;
mod panic;

use boot_args::BootArgs;
use serial::print;

#[no_mangle]
pub extern fn entry(boot_args: &BootArgs) -> ! {
    serial::init();

    let screen = unsafe {
        core::slice::from_raw_parts_mut(0xb8000 as *mut u8, 80 * 25 * 2)
    };

    /*
    {
        let mut pmem = boot_args.free_memory.lock();
        let pmem = pmem.as_mut().unwrap();
        //print!("{:?}\n", pmem);
    }
    */

    serial::print!("Apples\n");

    screen[..16].copy_from_slice(b"AAAAAAAAAAAAAAAA");

    screen[0] = 0x31;
    cpu::halt();
}
