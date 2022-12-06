#![no_std]
#![no_main]
use core::alloc::Layout;
use core::mem::size_of;

pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_WRITE: u64 = 1 << 1;
pub const PAGE_USER: u64 = 1 << 2;
pub const PAGE_NX: u64 = 1 << 63;

/// A strongly type physical address. This is effectively just and integer, but we have strongly
/// types it to make code clarity a bit higher. This may represent a host physical address, or a
/// guest physical address.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PhysAddr(pub u64);

/// A strongly typed virtual address
#[derive(Clone, Copy)]
#[repr(C)]
pub struct VirtAddr(pub u64);

pub trait PhysMem {
    /// Provide a virtual address to memory which contains the raw physical memory at `paddr` for
    /// `size` bytes
    unsafe fn translate(&mut self, paddr: PhysAddr, size: usize) -> Option<*mut u8>;

    /// Allocate physical memory with a requested layout
    fn alloc_phys(&mut self, layout: Layout) -> Option<PhysAddr>;

    /// Same as `alloc_phys` byt the memory will be zeroed
    fn alloc_phys_zeroed(&mut self, layout: Layout) -> Option<PhysAddr> {
        // Creat an allocation
        let alc = self.alloc_phys(layout)?;

        // Zero it out
        unsafe {
            let bytes = self.translate(alc, layout.size())?;
            core::ptr::write_bytes(bytes, 0, layout.size());
        }

        Some(alc)
    }
}

/// Different page sizes for 4-level x86_64 paging
#[repr(u64)]
#[derive(Clone, Copy)]
pub enum PageSize {
    Page4K = 4096,
    Page2M = 2 * 1024 * 1024,
    Page1G = 1 * 1024 * 1024 * 1024,
}

/// A 64-bit x86 page table. This uses 4 level paging, the `PhysAddr` is the address of the top
/// level page table. This is effectively `cr3` with the VPID bits masked off.
#[repr(C)]
pub struct PageTable {
    /// The physical address of the top-level page table. This is typically the value in `cr3`,
    /// without the VPID bits.
    table: PhysAddr,
}

impl PageTable{
    /// Create a new emptry page table
    pub fn new<P: PhysMem>(phys_mem: &mut P) -> Option<PageTable> {
        let table = phys_mem.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;

        Some(PageTable {
            table,
        })
    }

    /// Create a new page table from an existing CR3
    pub fn from_cr3<P: PhysMem>(phys_mem: &mut P, cr3: u64) -> PageTable {
        // Return out the page table with the VPID bits masked off(Intel Manual 4-24 Vol3A)
        PageTable {
            table: PhysAddr(cr3 & !0xfff),
        }
    }

    /// Get the base address of the page table
    pub fn table(&self) -> PhysAddr {
        self.table
    }

    /// Create a page table entry initialized to `init` at `vaddr` using `page_type` as page size.
    /// `read`, `write` and `exec` will be used as the permission bits.
    ///
    /// If `init` is `Some`, it will be invoked with the current offset into the mapping and the
    /// return value from the closure will be used to initialize that byte.
    pub unsafe fn map<P: PhysMem>(
        &mut self,
        phys_mem: &mut P,
        vaddr: VirtAddr,
        page_type: PageSize,
        size: u64,
        read: bool,
        write: bool,
        exec: bool,
    ) -> Option<()> {
        self.map_init::<fn(u64) -> u8, P>(phys_mem, vaddr, page_type, size, read, write, exec, None)
    }

    /// Create a page table entry initialized to `init` at `vaddr` using `page_type` as page size.
    /// `read`, `write` and `exec` will be used as the permission bits.
    ///
    /// If `init` is `Some`, it will be invoked with the current offset into the mapping and the
    /// return value from the closure will be used to initialize that byte.
    pub unsafe fn map_init<F, P: PhysMem>(
        &mut self,
        phys_mem: &mut P,
        vaddr: VirtAddr,
        page_type: PageSize,
        size: u64,
        _read: bool,
        write: bool,
        exec: bool,
        init: Option<F>
    ) -> Option<()>  where F: Fn(u64) -> u8 {
        // Get the raw page size in bytes
        let page_size = page_type as u64;
        // Determine the mask of the page size
        let page_mask = page_size - 1;

        // Save off the original virtual address
        let orig_vaddr = vaddr;

        // Make sure that the virtual address is aligned to the page size request
        if size <= 0 || (vaddr.0 & page_mask) != 0 {
            return None;
        }

        // Compute the end virtual address of this mapping
        let end_vaddr = vaddr.0.checked_add(size - 1)?;

        // Go trough each page in this mapping
        for vaddr in (vaddr.0..=end_vaddr).step_by(page_size as usize) {
            // Allocate the page
            let page = phys_mem.alloc_phys(
                Layout::from_size_align(page_size as usize, page_size as usize).ok()?
            )?;

            let ent = page.0 | PAGE_PRESENT |
                if write { PAGE_WRITE } else { 0 } |
                if exec { 0 } else { PAGE_NX };

            if let Some(init) = &init {
                // Translate the page
                let bytes = phys_mem.translate(page, page_size as usize)?;
                // Get acces to the memory we just allocated
                let sliced =
                    core::slice::from_raw_parts_mut(bytes, page_size as usize);

                for (off, byte) in sliced.iter_mut().enumerate() {
                    *byte = init(vaddr - orig_vaddr.0 + off as u64);
                }
            }

            // Add this mapping to the page table
            self.map_raw(phys_mem, VirtAddr(vaddr), page_type, ent, true, false, false);
        }

        Some(())
    }

    /// Map a `vaddr` to a raw page table entry `raw`. This will use the page size specified by
    /// `page_type`.
    ///
    /// * `vaddr` - Virtual address to create the mapping at
    /// * `page_type` - The page size to be used for the entry
    /// * `raw` - The raw page table entry to use
    /// * `add` - If true, will create page tables if need during translation
    /// * `update` - If `true`, the page table entry will be overwritten if it is already present.
    ///             If `false`, this will not update an already present mapping
    /// * `invlpg_on_update` - If an update of an exisiting page table entry occurs, and this is
    ///                     `true`, then an `invlp` will be executed to invalidate the TLBs for the
    ///                     virtual address.
    pub unsafe fn map_raw<P: PhysMem>(&mut self, phys_mem: &mut P, vaddr: VirtAddr, page_type: PageSize, raw: u64, add: bool,
        update: bool, invlpg_on_update: bool,
    ) -> Option<()> {
        // Get the raw page size in bytes
        let page_size = page_type as u64;
        // Determine the mask of the page size
        let page_mask = page_size - 1;

        // Make sure that the virtual address is aligned to the page size request and canonical
        if (vaddr.0 & page_mask) != 0 || cpu::canonicalize_address(vaddr.0) != vaddr.0 {
            return None;
        }

        // Compute the indexes for each level of the page table for this virtual address
        let mut indicies = [0; 4];
        let indicies = match page_type {
            PageSize::Page4K => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                indicies[2] = (vaddr.0 >> 21) & 0x1ff;
                indicies[3] = (vaddr.0 >> 12) & 0x1ff;
                &indicies[..4]
            }
            PageSize::Page2M => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                indicies[2] = (vaddr.0 >> 21) & 0x1ff;
                &indicies[..3]
            }
            PageSize::Page1G => {
                indicies[0] = (vaddr.0 >> 39) & 0x1ff;
                indicies[1] = (vaddr.0 >> 30) & 0x1ff;
                &indicies[..2] 
            }
        };

        // Go through each levle in the page table
        let mut table = self.table;

        let paddr_size = size_of::<u64>();
        for (depth, &index) in indicies.iter().enumerate() {
            // Get the physical address of the page table entry
            let ptp = PhysAddr(table.0 + index * paddr_size as u64);
            let vad = phys_mem.translate(ptp, paddr_size)?;

            // Get the page table entry
            let mut ent = *(vad as *const u64);

            // If we're not at the last level page table entry, and this is not present. Allocate
            // a new page table entry such that we can keep traversing
            if depth != indicies.len() - 1 && (ent & PAGE_PRESENT) == 0 {
                // We need to add a page table at this level
                if !add {
                    // It was requested that we do not add page table during the traversal if
                    // needed
                    return None;
                }

                // Allocate a new table in memory
                let new_table =
                    phys_mem.alloc_phys_zeroed(Layout::from_size_align(4096, 4096).ok()?)?;

                // Update the entry
                ent = new_table.0 | PAGE_USER | PAGE_WRITE | PAGE_PRESENT;
                *(vad as *mut u64) = ent;
            }

            // Check is this is the final level, if it is, this is what need to updated with raw
            if depth == indicies.len() - 1 && ((ent & PAGE_PRESENT) == 0 || update) {
                if (ent & PAGE_PRESENT) != 0 {
                    // We're overwriting a page table entry, we must be able to `invlpg`
                    *(vad as *mut u64) = raw;

                    // If we were requested to `invlpg` on updates, and it is physically possible
                    // that this page table is actually being used, Then `invlpg`
                    if invlpg_on_update && size_of::<VirtAddr>() == size_of::<usize>() {
                        // We caused and update, thus we need to invalidate page cache
                        cpu::invlpg(vaddr.0 as usize);
                    }
                } else {
                    // Creating a new page table entry
                    *(vad as *mut u64) = raw;
                }
                return Some(());
            } else if depth == indicies.len() - 1 {
                // All done translating, we were unable to update the entry due to `update` not
                // being set
                return None;
            }

            // Get the next level table
            table = PhysAddr(ent & 0xf_ffff_ffff_f000);
        }

        unreachable!();
    }
}
