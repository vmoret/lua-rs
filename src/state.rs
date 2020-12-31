//! Lua state.
use std::{cell::Cell, ffi::{CStr, CString}, fmt, ptr::{NonNull, null}};

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
            // Close all active to-be-closed variables in the main thread, release all objects in 
            // the given Lua state (calling the corresponding garbage-collection metamethods, if
            // any), and frees all dynamic memory used by this state.
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

    /// Pushes a **nil** value onto the stack.
    pub fn push_nil(&mut self) {
        unsafe { ffi::lua_pushnil(self.as_ptr()) }
    }

    /// Pushes a boolean value with value `t` onto the stack.
    pub fn push_boolean<T: Into<bool>>(&mut self, t: T) {
        let b = if t.into() { 1 } else { 0 };
        unsafe { ffi::lua_pushboolean(self.as_ptr(), b) }
    }

    /// Pushes a float with value `t` onto the stack.
    pub fn push_number<T: Into<f64>>(&mut self, t: T) {
        unsafe { ffi::lua_pushnumber(self.as_ptr(), t.into()) }
    }

    /// Pushes an integer with value `t` onto the stack.
    pub fn push_integer<T: Into<i64>>(&mut self, t: T) {
        unsafe { ffi::lua_pushinteger(self.as_ptr(), t.into()) }
    }

    /// Pushes the string `s` onto the stack.
    /// 
    /// Lua will make or reuse an internal copy of the given string, so the memory at `s` can be
    /// freed or reused immediately after the function returns. The string can contain any binary
    /// data, including embedded zeros.
    ///
    /// Returns a reference to the internal copy of the string.
    pub fn push_string<'a, S: AsRef<[u8]>>(&'a mut self, s: S) -> Result<&'a [u8]> {
        let s = s.as_ref();
        let s = unsafe {
            let cs = ffi::lua_pushlstring(self.as_ptr(), s.as_ptr() as *const i8, s.len());
            if cs.is_null() {
                return Err(Error::new(ErrorKind::InvalidData, "unexpected NULL while pushing string"));
            }
            let len = libc::strlen(cs);
            let data = cs as *const u8;
            std::slice::from_raw_parts(data, len)
        };
        Ok(s)
    }

    /// Ensures that the stack has space for at least `n` extra elements, that is, that you can
    /// safely push up to `n` values into it. It returns `false` if it cannot fulfill the request,
    /// either because it would cause the stack to be greater than a fixed maximum size (typically
    /// at least several thousand elements) or because it cannot allocate memory for the extra space.
    /// This function never shrinks the stack; if the stack already has space for the extra elements,
    /// it is left unchanged.
    pub fn check_stack(&mut self, n: i32) -> bool {
        unsafe { ffi::lua_checkstack(self.as_ptr(), n) != 0 }
    }

    /// Grows the stack size to `top + sz` elements, raising an error if the stack cannot grow to
    /// that size. `msg` is an additional text to go into the error message (or `None` for no
    /// additional text).
    pub fn try_check_stack(&mut self, sz: i32, msg: &str) {
        let msg = if !msg.is_empty() {
            let cs = CString::new(msg).unwrap();
            cs.as_ptr()
        } else {
            null()
        };
        unsafe { ffi::luaL_checkstack(self.as_ptr(), sz, msg) }
    }

    /// Returns `true` if the value at the given `index` is a boolean.
    pub fn is_boolean(&self, index: i32) -> bool {
        unsafe { ffi::lua_isboolean(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a C function.
    pub fn is_c_function(&self, index: i32) -> bool {
        unsafe { ffi::lua_iscfunction(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a function (either C or Lua).
    pub fn is_function(&self, index: i32) -> bool {
        unsafe { ffi::lua_isfunction(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is an integer (that is, the value is a
    /// number and is represented as an integer).
    pub fn is_integer(&self, index: i32) -> bool {
        unsafe { ffi::lua_isinteger(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a light userdata.
    pub fn is_light_userdata(&self, index: i32) -> bool {
        unsafe { ffi::lua_islightuserdata(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is **nil**.
    pub fn is_nil(&self, index: i32) -> bool {
        unsafe { ffi::lua_isnil(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the given `index` is not valid.
    pub fn is_none(&self, index: i32) -> bool {
        unsafe { ffi::lua_isnone(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the given `index` is not valid or if the value at this index is **nil**.
    pub fn is_none_or_nil(&self, index: i32) -> bool {
        unsafe { ffi::lua_isnone(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is number or a string convertible to a
    /// number.
    pub fn is_number(&self, index: i32) -> bool {
        unsafe { ffi::lua_isnumber(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a string or a number (which is always
    /// convertible to a string).
    pub fn is_string(&self, index: i32) -> bool {
        unsafe { ffi::lua_isstring(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a table.
    pub fn is_table(&self, index: i32) -> bool {
        unsafe { ffi::lua_istable(self.as_ptr(), index) != 0 }
    }
    
    /// Returns `true` if the value at the given `index` is a thread.
    pub fn is_thread(&self, index: i32) -> bool {
        unsafe { ffi::lua_isthread(self.as_ptr(), index) != 0 }
    }

    /// Returns `true` if the value at the given `index` is a userdata (either full or light).
    pub fn is_userdata(&self, index: i32) -> bool {
        unsafe { ffi::lua_isuserdata(self.as_ptr(), index) != 0 }
    }

    /// Converts the Lua value at the given `index` to a boolean.
    ///
    /// Like all tests in Lua, this returns `true` for any Lua value different from `false` and
    /// **nil**; otherwise it returns `false`.
    pub fn to_boolean(&self, index: i32) -> bool {
        unsafe { ffi::lua_toboolean(self.as_ptr(), index) != 0 }
    }

    /// Converts the Lua value at the given `index` to a byte slice.
    ///
    /// Returns a reference to the string inside the Lua state.
    pub fn as_bytes<'a>(&'a self, index: i32) -> &'a [u8] {
        unsafe {
            let mut len = 0;
            let ptr = ffi::lua_tolstring(self.as_ptr(), index, &mut len);
            let data = ptr as *const u8;
            std::slice::from_raw_parts(data, len)
        }
    }

    /// Converts the Lua value at the given `index` to a signed integer.
    pub fn to_integer<T: From<i64>>(&self, index: i32) -> Option<T> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tointegerx(self.as_ptr(), index, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            Some(n.into())
        }
    }

    /// Converts the Lua value at the given `index` to a float.
    pub fn to_number<T: From<f64>>(&self, index: i32) -> Option<T> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tonumberx(self.as_ptr(), index, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            Some(n.into())
        }
    }

    /// Converts the Lua value at the given `index` to a C string.
    pub fn as_c_str<'a>(&'a self, index: i32) -> &'a CStr {
        unsafe {
            CStr::from_ptr(ffi::lua_tostring(self.as_ptr(), index))
        }
    }

    pub fn info(&self, idx: i32) -> Info<'_> {
        unsafe {
            let state = self.as_ptr();

            let tp = ffi::lua_type(state, idx);

            let name = CStr::from_ptr(ffi::lua_typename(state, tp));

            Info { idx, tp, name }
        }
    }

    /// Returns the index of the top element in the stack.
    ///
    /// Because indices start at 1, this result is equal to the number of elements in the stack; in
    /// particular, 0 means an empty stack.
    pub fn top(&self) -> i32 {
        unsafe { ffi::lua_gettop(self.as_ptr()) }
    }

    /// Returns an iterator over the stack (from bottom to top).
    pub fn iter(&self) -> Iter<'_> {
        let top = self.top();
        Iter { state: self, n: 1, top }
    }

    /// Creates an iterator which dumps the values on a Lua stack from bottom to top.
    pub fn dump(&self) -> Dump<'_> {
        Dump { iter: self.iter() }
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

/// Immutable stack iterator.
pub struct Iter<'a> {
    state: &'a State,
    n: i32,
    top: i32,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (i32, i32, &'a State);
    fn next(&mut self) -> Option<Self::Item> {
        if self.n > self.top {
            return None;
        }

        let idx = self.n;
        self.n += 1;

        Some((idx, self.top, &self.state))
    }
}

/// An iterator that dumps the elements on a Lua stack.
pub struct Dump<'a> {
    iter: Iter<'a>,
}

/// Information of an element on the stack.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Info<'a> {
    idx: i32,
    tp: i32,
    name: &'a CStr,
}

impl<'a> Info<'a> {
    /// Returns the index of the element on the stack.
    pub fn index(&self) -> i32 {
        self.idx
    }

    /// Returns the code of the value type.
    pub fn type_code(&self) -> i32 {
        self.tp
    }

    /// Returns a reference to a C str with the name of the value type.
    pub fn type_name(&self) -> &'a CStr {
        self.name
    }
}

impl<'a> fmt::Display for Info<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} {:?}", self.idx, self.tp, self.name)
    }
}

impl<'a> Iterator for Dump<'a> {
    type Item = Info<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, _, state)| {
            state.info(idx)
        })
    }
}