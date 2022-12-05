use page_table::{PhysMem, PhysAddr, VirtAddr};
use core::alloc::{Layout, GlobalAlloc};
use rangeset::{Range, RangeSet};
use boot_args::BootArgs;

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
            serial::print!("Freed {} bytes\n", layout.size());
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
pub fn init(boot_args: &'static BootArgs) {
}
