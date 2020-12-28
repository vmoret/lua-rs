//! Lua memory allocation APIs

use std::{alloc, ptr};

use libc::c_void;

// Copied from https://github.com/rust-lang/rust/blob/master/src/libstd/sys_common/alloc.rs
#[cfg(all(any(
    target_arch = "x86",
    target_arch = "arm",
    target_arch = "mips",
    target_arch = "powerpc",
    target_arch = "powerpc64",
    target_arch = "asmjs",
    target_arch = "wasm32",
    target_arch = "hexagon"
)))]
const SYS_MIN_ALIGN: usize = 8;
#[cfg(all(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "mips64",
    target_arch = "s390x",
    target_arch = "sparc64",
    target_arch = "riscv64"
)))]
const SYS_MIN_ALIGN: usize = 16;

/// MemoryInfo keeps track of the memory being used by Lau.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemoryInfo {
    used: isize,
    limit: isize,
}

/// Allocate memory with the global allocator.
pub unsafe extern "C" fn alloc(
    ud: *mut c_void,
    ptr: *mut c_void,
    old_size: usize,
    new_size: usize,
) -> *mut c_void {
    let align = SYS_MIN_ALIGN;

    trace!("alloc() old_size={}, new_size={}", old_size, new_size);
    
    let info = &mut *(ud as *mut MemoryInfo);
    trace!("alloc() info={:?}", info);

    if new_size == 0 {
        // free memory
        if !ptr.is_null() {
            let layout = alloc::Layout::from_size_align_unchecked(old_size, align);
            alloc::dealloc(ptr as _, layout);
            info.used -= old_size as isize;
        }
        return ptr::null_mut();
    }

    // calculate memory difference
    let mut diff_size = new_size as isize;
    if !ptr.is_null() {
        diff_size -= old_size as isize;
    }

    if info.limit > 0 {
        // check if we're within memory limit
        if info.used + diff_size > info.limit {
            return ptr::null_mut();
        }
    }

    let new_layout = alloc::Layout::from_size_align_unchecked(new_size, align);

    if ptr.is_null() {
        // allocate new memory
        let p = alloc::alloc(new_layout);
        if !p.is_null() {
            info.used += diff_size;
        }
        return p as *mut c_void;
    }

    // reallocate memory
    let old_layout = alloc::Layout::from_size_align_unchecked(old_size, align);
    let p = alloc::realloc(ptr as _, old_layout, new_size);

    if !p.is_null() {
        info.used += diff_size;
    } else if !ptr.is_null() && new_size < old_size {
        // should not happen, still ...
        alloc::handle_alloc_error(new_layout);
    }

    p as *mut c_void
}

impl MemoryInfo {
    /// Creates a new `MemoryInfo` with a specific memory `limit`.
    pub fn new(limit: usize) -> Self {
        Self {
            limit: limit as isize,
            used: 0,
        }
    }
}
