//! A basic 8250A serial driver for x86
#![feature(rustc_private)]
#![no_std]

/// A collection of 4 8250A serial ports, as seen on IBM PC systems 
/// There are the 4 serail ports which are identified by the BIOS, and thus it is limited to just
/// COM1-COM4
#[repr(C)]
pub struct SerialPort {
    devices: [Option<u16>; 4],
}

/// Initialize all found serial ports for use at 115200 baud, no parity, 1 stop bit and return the
/// identified devices.
pub fn init() -> SerialPort {
    unsafe { SerialPort::new() }
}

impl SerialPort {
    /// Initialize the serial ports on the system to 115200n1. This should only be ever called once
    /// This also assumes that the memory is identity mapped, such that 0x400 is a valid pointer to
    /// the data area
    pub unsafe fn new() -> Self {
        let mut ret = SerialPort {
            devices: [None; 4]
        };

        for (com_id, device) in ret.devices.iter_mut().enumerate() {
            // Get the COM port I/O address from the BIOS Data Area(BDA)
            let port = unsafe { *(0x400 as *const u16).offset(com_id as isize) };

            // If the port address is zero, it is not present as reported by the BIOS 
            if port == 0 {
                // Serial port is not present
                *device = None;
                continue;
            }

            // Initialize the serial port a known state
            cpu::out8(port + 1, 0x00); // Disable all interrupts
            cpu::out8(port + 3, 0x80); // Enable DLAB
            cpu::out8(port + 0, 0x01); // Set divisor to 1 (lo byte), 115200 baud
            cpu::out8(port + 1, 0x00); // Set Hi byte divisor
            cpu::out8(port + 3, 0x03); // 8 bits, no parity, one stop bit
            cpu::out8(port + 4, 0x0B); // RTS/DSR set

            // Identify that we found an initialized a serial port
            *device = Some(port);
        }

        ret
    }

    // Write a byte to a COM port
    // Taking a mutable self tells us we have exclusive acess to the lock
    pub fn write_byte(&mut self, port: usize, byte: u8) {
        // Write a CR prior to all LFs
        if byte == b'\n' { self.write_byte(port, b'\r'); }

        // Check if this COM port exists
        if let Some(&Some(port)) = self.devices.get(port) {
            unsafe {
                // Wait for the output buffer to be ready
                while (cpu::in8(port + 5) & 0x20) == 0 {}

                // Write the bytes
                cpu::out8(port, byte);
            }
        }
    }

    /// Write bytes to all known serial devices
    pub fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            for com_id in 0..self.devices.len() {
                self.write_byte(com_id, byte);
            }
        }
    }
}
