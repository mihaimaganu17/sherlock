//! A basic 8250A serial driver for x86
#![no_std]

use lockcell::LockCell;
use cpu::{in8, out8};

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
    let mut serial = SERIAL.lock();


    for (com_id, device) in serial.devices.iter_mut().enumerate() {
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

unsafe fn write_byte(port: u16, byte: u8) {
    // Write a CR prior to all LFs
    if byte == b'\n' { write_byte(port, b'\r'); }

    // Wait for the output buffer to be ready
    while (cpu::in8(port + 5) & 0x20) == 0 {}

    // Write the bytes
    cpu::out8(port, byte);
}

/// Write bytes to all known serial devices
pub fn write(bytes: &[u8]) {
    // Get access to the serial ports
    let serial = SERIAL.lock();

    for &byte in bytes {
        for device in &serial.devices {
            if let Some(port) = *device {
                unsafe { 
                    write_byte(port, byte)
                }
            }
        }
    }
}

struct SerialWriter;

impl core::fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Ok(())
    }
}
