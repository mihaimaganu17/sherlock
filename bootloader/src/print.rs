/// Dummy type to implement `core::fmt::Write` for `print` macros
pub struct SerialWriter;
use crate::BOOT_ARGS;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if let Some(serial) = BOOT_ARGS.serial.lock().as_mut() {
            serial.write(s.as_bytes());
        }
        Ok(())
    }
}


// Print macro implementation
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let _ = core::fmt::Write::write_fmt(
            &mut $crate::print::SerialWriter,
            format_args!($($arg)*)
        );
    }
}
