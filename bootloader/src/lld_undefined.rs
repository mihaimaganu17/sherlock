extern crate compiler_builtins;

/// libc `memcpy` implementation in Rust
#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *const u8, n: usize)
        -> *mut u8 {
    compiler_builtins::mem::memcpy(dest, src, n)
}

/// libc `memmove` implementation in Rust
#[no_mangle]
pub unsafe extern fn memmove(dest: *mut u8, src: *const u8, n: usize)
        -> *mut u8 {
    compiler_builtins::mem::memmove(dest, src, n)
}

/// libc `memset` implementation in Rust
#[no_mangle]
pub unsafe extern fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    compiler_builtins::mem::memset(s, c, n)
}

/// libc `memcmp` implementation in Rust
#[no_mangle]
pub unsafe extern fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    compiler_builtins::mem::memcmp(s1, s2, n)
}

// ---------------------------------------------------------------------------
// Microsoft specific intrinsics
//
// These intrinsics use the stdcall convention however are not decorated
// with an @<bytes> suffix. To override LLVM from appending this suffix we
// have an \x01 escape byte before the name, which prevents LLVM from all
// name mangling.
// ---------------------------------------------------------------------------

/// Perform n % d
#[export_name="__aullrem"]
pub extern "stdcall" fn __aullrem(n: u64, d: u64) -> u64 {
    compiler_builtins::int::udiv::__umoddi3(n, d)
}

/// Perform n / d
#[export_name="__aulldiv"]
pub extern "stdcall" fn __aulldiv(n: u64, d: u64) -> u64 {
    compiler_builtins::int::udiv::__udivdi3(n, d)
}

/// Perform n % d
#[export_name="___allrem"]
pub extern "stdcall" fn __allrem(n: i64, d: i64) -> i64 {
    compiler_builtins::int::sdiv::__moddi3(n, d)
}

/// Perform n / d
#[export_name="__alldiv"]
pub extern "stdcall" fn __alldiv(n: i64, d: i64) -> i64 {
    compiler_builtins::int::sdiv::__divdi3(n, d)
}


#[export_name="_fltused"]
pub static FLTUSED: usize = 0;
