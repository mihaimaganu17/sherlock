/// This file is used to hold and access all of the core locals

use core::sync::atomic::{AtomicUsize, Ordering};
use boot_args::BootArgs;

const _GS_BASE: u64 = 0xC000_0101;

/// A counter of all cores online
static CORES_ONLINE: AtomicUsize = AtomicUsize::new(0);

/// A core exclusive data structure which can be accessed via the `core!()` macro.
///
/// This structure must be `Sync` since the same core locals will be used during an interrupt on
/// this core.
pub struct CoreLocals {
    /// A pointer to ourself
    pub address: usize,

    /// A unique, sequentially allocated identifier for this core
    pub id: usize,

    /// A reference to the bootloader arguments.
    pub boot_args: &'static BootArgs,
}

/// Empty marker trait that requires `Sync`, such that we can compile-time assert that `CoreLocals`
/// is `Sync`
trait CoreGuard: Sync + Sized {}
impl CoreGuard for CoreLocals {}

/// A shorcut to `get_core_locals`
#[macro_export]
macro_rules! core {
    () => {
        $crate::core_locals::get_core_locals()
    }
}

/// Get a reference to the current core locals
pub fn get_core_locals() -> &'static CoreLocals {
    unsafe {
        let ptr: usize;

        // Get the first `u64` from `CoreLocals`
        core::arch::asm!(r#"mov {}, gs:[0]"#, out(reg) ptr); 

        &*(ptr as *const CoreLocals)
    }
}

/// Initialize the locals for this core
pub fn init(boot_args: &'static BootArgs) {
    // Get access to the physical memory allocator
    let mut pmem = boot_args.free_memory.lock();
    let pmem = pmem.as_mut().unwrap();

    // Allocate the core locals
    let core_locals_ptr = pmem.allocate(
        core::mem::size_of::<CoreLocals>() as u64,
        core::mem::align_of::<CoreLocals>() as u64,
    ).unwrap();

    // Construct the core locals
    let core_locals = CoreLocals {
        address: core_locals_ptr,
        id: CORES_ONLINE.fetch_add(1, Ordering::SeqCst), 
        boot_args: boot_args,
    };

    unsafe { 
        // Move the core locals into the allocation
        core::ptr::write(core_locals_ptr as *mut CoreLocals, core_locals);

        cpu::set_gs_base(core_locals_ptr as u64);
    }
}
