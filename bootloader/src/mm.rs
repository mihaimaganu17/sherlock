use core::convert::TryInto;
use core::alloc::{Layout, GlobalAlloc};
use page_table::{PhysAddr, PhysMem};
use crate::realmode::{RegisterState, invoke_realmode};
use rangeset::{RangeSet, Range};
use crate::BOOT_ARGS;

pub struct PhysicalMemory<'a>(pub &'a mut RangeSet);

impl<'a> PhysMem for PhysicalMemory<'a> {
    unsafe fn translate(&mut self, paddr: PhysAddr, size: usize) -> Option<*mut u8> {
        // Can't translate for a 0 size access
        if size <= 0 {
            return None;
        }
        // Convert the physical address into a `usize` which is addresssable in the bootloader
        let paddr: usize = paddr.0.try_into().ok()?;
        let _pend: usize = paddr.checked_add(size - 1)?;
        
        // At this point, `paddr` is for `size` bytes fits in the 32-bit address space we have
        // mapped in!
        Some(paddr as *mut u8)
    }

    fn alloc_phys(&mut self, layout: Layout) -> Option<PhysAddr> {
        self.0.allocate(layout.size() as u64, layout.align() as u64)
            .map(|x| PhysAddr(x as u64))
    }
}

/// Global allocator
#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

/// The global allocator for the bootloader, this just uses physical memory as a backing and does
/// not handle any fancy things like fragmentation.
struct GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Get access to physical memory
        let mut pmem = BOOT_ARGS.free_memory.lock();

        pmem.as_mut().and_then(|x| {
            x.allocate(layout.size() as u64, layout.align() as u64)
        }).unwrap_or(0) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // We have nothing to free for a zero-size-type
        if layout.size() <= 0 { return; }
        // Get access to physical memory
        let mut pmem = BOOT_ARGS.free_memory.lock();

        pmem.as_mut().and_then(|x| {
            let end = (ptr as u64).checked_add(layout.size().checked_sub(1)? as u64)?;
            x.insert(Range { start: ptr as u64, end });
            Some(())
        }).expect("Cannot free memory without initialized MM");
    }
}

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    panic!("Out of memory");
}

/// Initialize the physical memory manager. Here we get the memory map from the BIOS via E820 and
/// put it into a `RangeSet` for tracking and allocation. We also subtract off the first 1 MiB of
/// memory to prevent BIOS data structures from being overwritten.
pub fn init() {
    let mut pmem = BOOT_ARGS.free_memory.lock();

    // If physical memory has already been initialized, just return out
    if pmem.is_some() {
        return;
    }

    // Create a new empty `RangeSet` for tracking free physical memory
    let mut free_memory = RangeSet::new();

    // Loop through the memory the BIOS reports twice. The first time we accumulate all of the
    // memory that is marked as free. The second pass we remove all ranges that are not
    // marked as free. This sanitizes the BIOS memory map, and makes sure that any memory
    // marked both free and non-free, is not marked free at all.
    for &add_free_mem in &[true, false] {
        // Allocate a register state to use when doing the E820 call
        let mut regs = RegisterState::default();

        // Set the continuation code to 0 for the first E820 call
        regs.ebx = 0;
        loop {
            // http://www.uruk.org/orig-grub/mem64mb.html
            #[repr(C)]
            #[derive(Default, Debug)]
            struct E820Entry {
                base: u64,
                size: u64,
                typ: u32,
            }

            // Create a zeroed out E820 entry
            let mut entry = E820Entry::default();

            // Set upt the arguments for E820, we use the prvious continuation code
            regs.eax = 0xe820;
            regs.edi = &mut entry as *mut E820Entry as u32;
            regs.ecx = core::mem::size_of_val(&entry) as u32;
            regs.edx = u32::from_be_bytes(*b"SMAP");

            // Invoke the BIOS for the E820 memory map
            unsafe { invoke_realmode(0x15, &mut regs); }

            if (regs.efl & 1) != 0 {
                // CF is set, which means an error
                panic!("Error reported by BIOS on E820");
            }

            // If the entry is free, mark the memory as free
            if add_free_mem && entry.typ == 1 && entry.size > 0 {
                free_memory.insert(Range {
                    start: entry.base,
                    end: entry.base.checked_add(entry.size - 1).unwrap(),
                });
            } else if !add_free_mem && entry.typ != 1 && entry.size > 0 {
                free_memory.remove(Range {
                    start: entry.base,
                    end: entry.base + entry.size - 1
                });
            }

            if regs.ebx == 0 {
                // Last entry
                break;
            }
        }
    }

    // Remove the IVT and BDA being marked as free so we do not overwrite them
    free_memory.remove(Range {
        start: 0,
        end: 1024 * 1024 - 1,
    });

    *pmem = Some(free_memory);
}
