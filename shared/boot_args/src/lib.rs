//! Contains a common structure definition that can be passed from the bootloader to the kernel.
//! This contains any information that the bootloader wants to enlighten the kernel with.
//!
//! This structure also supports backwards passing, for the kernel to pass information back to the
//! bootloader. This just means that this structure lives forever and is not deleted or moved by
//! either the bootloader or kernel.
#![no_std]

use rangeset::RangeSet;
use lockcell::LockCell;

#[repr(C)]
pub struct BootArgs {
    /// All memory which is available for use by the kernel. This structure is potentially used at
    /// the same time by both the bootloader and the kernel.
    pub free_memory: LockCell<Option<RangeSet>>,
}

