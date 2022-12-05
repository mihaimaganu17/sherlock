//! The main kernel entry point
#![feature(alloc_error_handler, panic_info_message)]
#![no_std]
#![no_main]

extern crate core_reqs;
#[macro_use] mod core_locals;
#[macro_use] mod print;
mod panic;

use boot_args::BootArgs;
use serial::SerialPort;

#[no_mangle]
pub extern fn entry(boot_args: &'static BootArgs) -> ! {
    // Create the serail port driver
    let mut _serial = unsafe { SerialPort::new() };
    // Initialize the corelocals
    core_locals::init(boot_args);

    if core!().id == 0 {
        // One-time initialization for the whole kernel and all the cores

        // Initialize the serial port
        serial::init();
    }

    print!("{}\n", core!().id);

    {
        let mut pmem = boot_args.free_memory.lock();
        let pmem = pmem.as_mut().unwrap();
        print!("{:?}\n", pmem);
    }

    cpu::halt();
}
