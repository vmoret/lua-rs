//! Lua state.
use std::{cell::Cell, fmt, ptr::NonNull};
use crate::{alloc, ffi};

/// A soft limit on the amount of references that may be made to a `State`.
///
/// Going above this limit will abort your program (although not
/// necessarily) at _exactly_ `MAX_REFCOUNT + 1` references.
const MAX_REFCOUNT: usize = (isize::MAX) as usize;

/// The type returned by [`State::value_type`] when a non-valid but acceptable index was provided.
#[allow(dead_code)]
pub const LUA_TNONE: i32 = ffi::LUA_TNONE;

/// The **nil** value type.
pub const LUA_TNIL: i32 = ffi::LUA_TNIL;

/// The *number* value type.
pub const LUA_TNUMBER: i32 = ffi::LUA_TNUMBER;

/// The *boolean* value type.
pub const LUA_TBOOLEAN: i32 = ffi::LUA_TBOOLEAN;

/// The *string* value type.
pub const LUA_TSTRING: i32 = ffi::LUA_TSTRING;

/// The *table* value type.
pub const LUA_TTABLE: i32 = ffi::LUA_TTABLE;

/// The *function* value type.
pub const LUA_TFUNCTION: i32 = ffi::LUA_TFUNCTION;

/// The *user data* value type.
pub const LUA_TUSERDATA: i32 = ffi::LUA_TUSERDATA;

/// The *thread* value type.
pub const LUA_TTHREAD: i32 = ffi::LUA_TTHREAD;

/// The *light user data* value type.
pub const LUA_TLIGHTUSERDATA: i32 = ffi::LUA_TLIGHTUSERDATA;

// This is repr(C) to future-proof against possible field-reordering.
#[repr(C)]
struct StateBox {
    rc: Cell<usize>,
    ptr: NonNull<ffi::lua_State>,
}

impl fmt::Pointer for StateBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr.as_ptr(), f)
    }
}

impl StateBox {
    fn count(&self) -> usize {
        self.rc.get()
    }

    fn inc_count(&self) {
        let count = self.count();

        // We want to abort on overflow instead of dropping the value.
        // The reference count will never be zero when this is called;
        // nevertheless, we insert an abort here to hint LLVM at
        // an otherwise missed optimization.
        if count == 0 || count == MAX_REFCOUNT {
            std::process::abort();
        }
        self.rc.set(count + 1);
        trace!("State({:p}) increased ref counter to {}", self, self.rc.get());
    }

    fn dec_count(&self) {
        self.rc.set(self.count() - 1);
        trace!("State({:p}) decreased ref counter to {}", self, self.rc.get());
    }
}

impl Drop for StateBox {
    fn drop(&mut self) {
        debug!("{:p} close", self.ptr.as_ptr());
        unsafe { 
            ffi::lua_close(self.ptr.as_ptr())
        }
    }
}

/// The Lua state.
pub struct State {
    ptr: *mut StateBox,
}

unsafe impl Send for State {}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State({:p})", self.inner().ptr)
    }
}

impl fmt::Pointer for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner().ptr, f)
    }
}

impl State {
    /// Creates a new `State` without a memory allocation limit.
    pub fn new() -> Self {
        Self::with_limit(0)
    }

    /// Creates a new `State` with the given memory allocation `limit`.
    pub fn with_limit(limit: usize) -> Self {
        let ptr = new_state_unchecked(limit);
        let b = box StateBox { ptr, rc: Cell::new(1) };
        let ptr = Box::into_raw(b);
        Self { ptr }
    }

    fn inner(&self) -> &StateBox {
        // SAFETY: This unsafety is ok because while this `State` is alive we're
        // guaranteed that the inner pointer is valid.
        unsafe { &(*self.ptr) }
    }

    /// Gets a mutable pointer to the Lua state pointer.
    #[inline]
    pub fn as_ptr(&self) -> *mut ffi::lua_State {
        self.inner().ptr.as_ptr()
    }

    /// Ensures that the stack has space for at least `n` extra elements, that is, that you can
    /// safely push up to `n` values into it. It returns `false` if it cannot fulfill the request,
    /// either because it would cause the stack to be greater than a fixed maximum size (typically
    /// at least several thousand elements) or because it cannot allocate memory for the extra space.
    /// 
    /// This function never shrinks the stack; if the stack already has space for the extra elements,
    /// it is left unchanged.
    pub fn check_stack(&self, n: i32) -> bool {
        unsafe { ffi::lua_checkstack(self.as_ptr(), n) != 0 }
    }

    /// Returns the index of the top element in the stack.
    /// 
    /// Because indices start at 1, this result is equal to the number of elements in the stack; in 
    /// particular, 0 means an empty stack.
    pub fn get_top(&self) -> i32 {
        unsafe { ffi::lua_gettop(self.as_ptr()) }
    }

    /// Accepts any `index`, or 0, and sets the stack top to this `index`. If the new top is greater
    /// than the old one, then the new elements are filled with **nil**. If `index` is 0, then all
    /// stack elements are removed.
    /// 
    /// This function can run arbitrary code when removing an index marked as to-be-closed from the
    /// stack.
    pub fn set_top(&self, index: i32) {
        trace!("set_top() index = {}", index);
        unsafe { ffi::lua_settop(self.as_ptr(), index) }
    }

    /// Returns the type of the value in the given valid `index`, or [`LUA_TNONE`] for a non-valid
    /// but acceptable index.
    pub fn value_type(&self, index: i32) -> i32 {
        unsafe { ffi::lua_type(self.as_ptr(), index) }
    }
}

/// Creates a new `NonNull<ffi::lua_State>` with the given memory allocation `limit`.
/// 
/// # Panics
///
/// Panics when the ptr is non-null.
fn new_state_unchecked(limit: usize) -> NonNull<ffi::lua_State> {
    // initialize Lua user data
    let ud = Box::into_raw(box alloc::MemoryInfo::new(limit));

    // initialize raw Lua state
    let ptr = unsafe { ffi::lua_newstate(alloc::alloc, ud as _) };
    debug!("{:p} new state", ptr);

    // panic when pointer is null, that is when not enough memory could be
    // allocated for the Lua state.
    if ptr.is_null() {
        panic!("failed to allocate enough memory for a Lua state");
    }

    // SAFETY: This unsafety is ok becuase the pointer is already checked and
    // is non-null.
    unsafe { NonNull::new_unchecked(ptr) }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        debug!("{:p} clone state (rc = {})", self.inner().ptr, self.inner().count());
        self.inner().inc_count();
        Self { ptr: self.ptr }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        debug!("{:p} drop state (rc = {})", self.inner().ptr, self.inner().count());
        self.inner().dec_count();
        if self.inner().count() == 0 {
            unsafe {
                // SAFETY: This safety is ok becuase while this `State` is alive
                // we're guaranteed that the inner pointer was not freed before.
                Box::from_raw(self.ptr);
            }
        }
    }
}

impl AsRef<State> for State {
    fn as_ref(&self) -> &State {
        self
    }
}
