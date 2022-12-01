#![feature(rustc_private)]
#![no_std]

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

#[no_mangle]
pub extern "C" fn __CxxFrameHandler3() {}

// ---------------------------------------------------------------------------
// Microsoft specific intrinsics
//
// These intrinsics use the stdcall convention however are not decorated
// with an @<bytes> suffix. To override LLVM from appending this suffix we
// have an \x01 escape byte before the name, which prevents LLVM from all
// name mangling.
// ---------------------------------------------------------------------------

#[export_name="_fltused"]
pub static FLTUSED: usize = 0;
