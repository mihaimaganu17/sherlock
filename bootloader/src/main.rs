#![feature(rustc_private, lang_items, panic_info_message)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use serial::print;

#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
    print!("PANIC:");

    if let Some(location) = panic_info.location() {
        print!(" {}:{}:{}", location.file(), location.line(), location.column());
    }

    if let Some(msg) = panic_info.message() {
        print!(" {:?}", msg);
    }

    print!("\n");
    cpu::halt();
}

/// All general purpose registers for 32-bit x86
#[repr(C)]
#[derive(Default, Debug)]
struct RegisterState {
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    efl: u32,

    es: u16,
    ds: u16,
    fs: u16,
    gs: u16,
    ss: u16,
}

extern {
    fn invoke_realmode(int_number: u8, regs: *const RegisterState);
}

#[no_mangle]
extern fn entry() {
    serial::init();

    unsafe {
        invoke_realmode(0x10, &RegisterState {
            eax: 0x0003,
            ..Default::default()
        });
    }

    print!("Hello world!\n");
    cpu::halt();
}

