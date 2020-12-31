//! Lua state.
use std::{cell::Cell, ffi::{CStr, CString}, fmt, ptr::NonNull};

use crate::{alloc, error::{Error, ErrorKind, Result}, ffi};

pub use types::*;

/// A soft limit on the amount of references that may be made to a `State`.
///
/// Going above this limit will abort your program (although not
/// necessarily) at _exactly_ `MAX_REFCOUNT + 1` references.
const MAX_REFCOUNT: usize = (isize::MAX) as usize;

pub mod types {
    use super::ffi;

    /// The type returned by [`Stack::value_type`](crate::Stack::value_type) when a non-valid but 
    /// acceptable index was provided.
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
}
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

/// A Lua state.
///
/// This is a single-threaded reference-counting pointer for a C `lua_State` structure, ensuring
/// that the Lua state is closed when, and only when, all references are dropped.
///
/// # Examples
///
/// ```
/// # extern crate lua;
/// use lua::State;
///
/// let state = State::default();
/// ```
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
    /// Creates a new `State` without a memory limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::State;
    ///
    /// let state = State::new();
    /// ```
    pub fn new() -> Self {
        Self::with_limit(0)
    }

    /// Constructs a new `State` with the specified memory limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::State;
    ///
    /// let state = State::with_limit(8 * 1_024);
    /// ```
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
    fn as_ptr(&self) -> *mut ffi::lua_State {
        self.inner().ptr.as_ptr()
    }

    /// Opens all standard Lua libraries into the given state.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::State;
    ///
    /// let mut state = State::new();
    /// state.open_libs();
    /// ```
    pub fn open_libs(&mut self) {
        unsafe { ffi::luaL_openlibs(self.as_ptr()) }
    }

    /// Loads a string as a Lua chunk. This function uses [`.load()`] to load the chunk in the
    /// provided data.
    /// 
    /// This function returns the same results as [`.load()`].
    ///
    /// Also as [`.load()`], this function only loads the chunk; it does not run it.
    ///
    /// [`.load()`]: State::load
    pub fn load_string<T: Into<Vec<u8>>>(&mut self, t: T) -> Result<()> {
        let s = CString::new(t)?;

        // load the zero terminated C string
        let code = unsafe { ffi::luaL_loadstring(self.as_ptr(), s.as_ptr()) };
        self.handle_result(code, ())
    }

    /// Calls a function (or a callable object) in protected mode.
    pub fn pcall(&mut self, nargs: i32, nresults: i32, msgh: i32) -> Result<()> {
        let code = unsafe { ffi::lua_pcall(self.as_ptr(), nargs, nresults, msgh) };
        self.handle_result(code, ())
    }

    /// Pops n elements from the stack.
    /// 
    /// This can run arbitrary code when removing an index marked as to-be-closed from the stack.
    pub fn pop(&mut self, n: i32) {
        unsafe { ffi::lua_pop(self.as_ptr(), n) }
    }

    /// Converts the Lua value at the given `index` to a C string.
    pub fn as_c_str<'a>(&'a self, index: i32) -> &'a CStr {
        unsafe {
            CStr::from_ptr(ffi::lua_tostring(self.as_ptr(), index))
        }
    }

    /// Returns a [`Result<T>`](crate::error::Result) based on provided result `code`.
    ///
    /// When `code` is not `LUA_OK` or `LUA_YIELD`, it will read the error code from the top of the
    /// stack and return an [`Err`], otherwise [`Ok`] is returned with the provided `value`.
    fn handle_result<T>(&self, code: i32, value: T) -> Result<T> {
        match code {
            ffi::LUA_OK | ffi::LUA_YIELD => Ok(value),
            errcode => {
                let errmsg = self.as_c_str(-1);
                let error = format!("{} (code = {})", errmsg.to_string_lossy(), errcode);
                Err(Error::new(ErrorKind::InvalidData, error))
            }
        }
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
