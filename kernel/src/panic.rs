use core::panic::PanicInfo;

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
