//! A basic 8250A serial driver for x86
#![no_std]

use lockcell::LockCell;
use cpu::out8;

/// A collection of 4 8250A serial ports, as seen on IBM PC systems 
/// There are the 4 serail ports which are identified by the BIOS, and thus it is limited to just
/// COM1-COM4
struct SerialPort {
    devices: [Option<u16>; 4],
}

/// Global state for the serial ports on the system
static SERIAL: LockCell<SerialPort> = LockCell::new(SerialPort {
    devices: [None; 4],
});

/// Initialize all found serial ports for use at 115200 baud, no parity, 1 stop bit
pub fn init() {
    // Get access to the serial ports
    SERIAL.lock().init();
}

impl SerialPort {
    /// Initialize the serial ports on the system to 115200n1
    fn init(&mut self) {
        for (com_id, device) in self.devices.iter_mut().enumerate() {
            // Get the COM port I/O address from the BIOS Data Area(BDA)
            let port = unsafe { *(0x400 as *const u16).offset(com_id as isize) };

            // If the port address is zero, it is not present as reported by the BIOS 
            if port == 0 {
                // Serial port is not present
                *device = None;
                continue;
            }

            unsafe {
                // Initialize the serial port a known state
                out8(port + 1, 0x00); // Disable all interrupts
                out8(port + 3, 0x80); // Enable DLAB
                out8(port + 0, 0x01); // Set divisor to 1 (lo byte), 115200 baud
                out8(port + 1, 0x00); // Set Hi byte divisor
                out8(port + 3, 0x03); // 8 bits, no parity, one stop bit
                out8(port + 4, 0x03); // RTS/DSR set
            }

            // Identify that we found an initialized a serial port
            *device = Some(port);
        }
    }

    // Write a byte to a COM port
    // Taking a mutable self tells us we have exclusive acess to the lock
    fn write_byte(&mut self, port: usize, byte: u8) {
        // Write a CR prior to all LFs
        if byte == b'\n' { self.write_byte(port, b'\r'); }

        // Check if this COM port exists
        if let Some(Some(port)) = self.devices.get(port) {
            unsafe {
                // Wait for the output buffer to be ready
                while (cpu::in8(port + 5) & 0x20) == 0 {}

                // Write the bytes
                cpu::out8(*port, byte);
            }
        }
    }

    /// Write bytes to all known serial devices
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            for com_id in 0..self.devices.len() {
                self.write_byte(com_id, byte);
            }
        }
    }
}

/// Dummy type to implement `core::fmt::Write` for `print` macros
pub struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        SERIAL.lock().write(s.as_bytes());
        Ok(())
    }
}

// Print macro implementation
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        let _ = core::fmt::Write::write_fmt(&mut $crate::SerialWriter, format_args!($($arg)*));
    }
}
