#![feature(rustc_private)]
#![feature(panic_info_message, alloc_error_handler, lang_items)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate core_reqs;

#[macro_use] mod print;
mod realmode;
mod mm;
mod panic;
mod pxe;

use core::sync::atomic::{AtomicU64, Ordering};
use boot_args::BootArgs;
use parse_pe::PeParser;
use page_table::{VirtAddr, PageTable, PageSize};
use lockcell::LockCell;
use serial::SerialPort;

/// Size to allocate for kernel stacks
const KERNEL_STACK_SIZE: u64 = 32 * 1024;

/// Padding deadspace to add between kernel stacks
const KERNEL_STACK_PAD: u64 = 32 * 1024;

/// Global arguments shared between the kernel and the bootloader. It is critical that every
/// structure in here is identical in shape between boot 64-bit and 32-bit representations.
pub static BOOT_ARGS: BootArgs = BootArgs {
    free_memory: LockCell::new(None),
    serial: LockCell::new(None),
    page_table: LockCell::new(None), 
    kernel_entry: LockCell::new(None),
    stack_vaddr: AtomicU64::new(0x0000_7473_0000_0000), // "st" in ascii LE
    print_lock: LockCell::new(()),
};

#[no_mangle]
pub extern fn entry() -> !{
    // Initialize the serial driver
    {
        let mut serial = BOOT_ARGS.serial.lock();

        if serial.is_none() {
            // Drive has not yet been set up, initialize the ports
            *serial = Some(unsafe { SerialPort::new() });

            core::mem::drop(serial);
            // Clear the screen
            for _ in 0..100 {
                print!("\n");
            }
        }
    }

    // Initialize the MMU
    mm::init();

    // Download the kernel and create the kernel page table
    let (entry_point, stack, cr3) = {
        let mut kernel_entry = BOOT_ARGS.kernel_entry.lock();
        let mut page_table = BOOT_ARGS.page_table.lock();

        // If no kernel entry is set yet, download the kernel and load it
        if kernel_entry.is_none() {
            assert!(page_table.is_none(), "Page table set up before kernel!?");

            // Download the kerneL
            let kernel = pxe::download("sherlock.kern")
                .expect("Failed to download chocolate_milk.kern over TFTP");

            // Parse the kernel PE
            let pe = PeParser::parse(&kernel).expect("Failed to parse PE");

            // Get exclusive access to physical memory
            let mut pmem = BOOT_ARGS.free_memory.lock();
            let pmem = pmem.as_mut().expect("Whoa, physical memory not init yet");
            let mut pmem = mm::PhysicalMemory(pmem);

            // Create a new page table
            let mut table = PageTable::new(&mut pmem).expect("Failed to create page table");

            // Make an identity map, because after we enable the CR3 paging we will not know where
            // we are in memory
            // Create a 2 GiB identity map
            for paddr in (0..(4u64 * 1024 * 1024 * 1024)).step_by(4096) {
                unsafe {
                    table
                        .map_raw(&mut pmem, VirtAddr(paddr), PageSize::Page4K, paddr | 3, true, false, false)
                        .unwrap();
                }
            }

            // Load all the sections from the PE into the page table
            pe.sections(|vaddr, vsize, raw, _, _, _| {
                // Create a new virtual mapping for the PE range and initialize it to the raw bytes
                // from the PE file, otherwise to zero for all the bytes that were not initialized in
                // the file
                unsafe {
                    table.map_init(&mut pmem, VirtAddr(vaddr), PageSize::Page4K, vsize as u64, true, true, true, 
                        Some(|off| raw.get(off as usize).copied().unwrap_or(0)))?;
                }
                print!("Created map at {:x?} for {:x?}\n", vaddr as usize, vsize as usize);
                Some(())
            }).unwrap();

            print!("Entry point is {:#x}\n", pe.entry_point);

            // Set upt the entry point and page table
            *kernel_entry = Some(pe.entry_point);
            *page_table = Some(table);
        }

        // Get exclusive access to physical memory
        let mut pmem = BOOT_ARGS.free_memory.lock();
        let pmem = pmem.as_mut().expect("Whoa, physical memory not init yet");
        let mut pmem = mm::PhysicalMemory(pmem);

        // At this point the page table is always set up
        let page_table = page_table.as_mut().unwrap();

        // Get a unique stack address for this core
        let stack_addr = BOOT_ARGS.stack_vaddr
            .fetch_add(KERNEL_STACK_SIZE + KERNEL_STACK_PAD, Ordering::SeqCst);

        // Map in a stack for every core
        unsafe {
            page_table.map(&mut pmem, VirtAddr(stack_addr), PageSize::Page4K, KERNEL_STACK_SIZE, true, true, false).unwrap();
        }

        (
            *kernel_entry.as_ref().unwrap(),
            stack_addr + KERNEL_STACK_SIZE,
            page_table.table().0 as u32,
        )
    };

    extern {
        fn enter64(entry_point: u64, stack: u64, param: u64, cr3: u32) -> !;
    }

    // Enter 64-bit long mode and the kernel
    unsafe {
        enter64(entry_point, stack, &BOOT_ARGS as *const BootArgs as u64, cr3); 
    }
}

