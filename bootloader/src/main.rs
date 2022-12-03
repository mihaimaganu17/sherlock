#![feature(rustc_private)]
#![feature(panic_info_message, alloc_error_handler, lang_items)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate core_reqs;

mod realmode;
mod mm;
mod panic;
mod pxe;

use boot_args::BootArgs;
use parse_pe::PeParser;
use page_table::{VirtAddr, PageTable, PageSize};
use lockcell::LockCell;

pub static BOOT_ARGS: BootArgs = BootArgs {
    free_memory: LockCell::new(None),
};

#[no_mangle]
extern fn entry() -> !{
    serial::init();
    mm::init();

    let (entry_point, stack, cr3) = {
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

        // Make an identity map, because after we enable the CR3 paging we will not know where we are
        // in memory
        // Create a 2 GiB identity map
        for paddr in (0..(2u64 * 1024 * 1024 * 1024)).step_by(4096) {
            unsafe {
                table
                    .map_raw(VirtAddr(paddr), PageSize::Page4K, paddr | 3, true, false, false)
                    .unwrap();
            }
        }

        // Load all the sections from the PE into the page table
        pe.sections(|vaddr, vsize, raw, _, _, _| {
            // Create a new virtual mapping for the PE range and initialize it to the raw bytes
            // from the PE file, otherwise to zero for all the bytes that were not initialized in
            // the file
            unsafe {
                table.map_init(VirtAddr(vaddr), PageSize::Page4K, vsize as u64, true, true, true, 
                    Some(|off| raw.get(off as usize).copied().unwrap_or(0)))?;
            }
            serial::print!("Created map at {:x?} for {:x?}\n", vaddr as usize, vsize as usize);
            Some(())
        }).unwrap();

        // Map in a stack
        unsafe {
            table.map(VirtAddr(0xb00_0000_0000), PageSize::Page4K, 8192, true, true, false).unwrap();
        }

        // Return the artifacts out
        (pe.entry_point, 0xb00_0000_0000 + 8192, table.table().0 as u32)
    };

    extern {
        fn enter64(entry_point: u64, stack: u64, param: u64, cr3: u32) -> !;
    }

    // Enter 64-bit long mode and the kernel
    unsafe {
        enter64(entry_point, stack, &BOOT_ARGS as *const BootArgs as u64, cr3); 
    }
}

