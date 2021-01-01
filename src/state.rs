//! Lua state.
use std::{
    ffi::{CStr, CString},
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
    ptr::{self, null, NonNull},
};

use crate::{
    alloc,
    error::{Error, ErrorKind, Result},
    ffi,
};

use libc::c_void;
pub use types::*;

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

pub type CFunction = unsafe extern "C" fn(*mut ffi::lua_State) -> i32;

pub trait Push {
    /// Pushes the value `p` onto the stack and returns the number of slots used.
    fn push(&self, state: &mut State) -> Result<i32>;
}

pub trait Pull {
    fn size() -> i32 {
        1
    }

    fn pull(state: &State, index: i32) -> Result<Self>
    where
        Self: Sized;

    fn pop(state: &mut State) -> Result<Self>
    where
        Self: Sized,
    {
        let ret = Self::pull(&state, -1);
        state.pop(Self::size());
        ret
    }
}

macro_rules! impl_primitives {
    ([$($ty:ty),*], $push:ident, $pull:ident) => {$(
        impl Push for $ty {
            fn push(&self, state: &mut State) -> Result<i32> {
                state.$push(*self);
                Ok(1)
            }
        }
        impl Pull for $ty {
            fn pull(state: &State, index: i32) -> Result<Self> {
                state.$pull(index).ok_or(Error::new(ErrorKind::InvalidData, "invalid number"))
            }
        }
    )*};
}

impl_primitives!([i64, i32, i16, i8, u32, u16, u8], push_integer, to_integer);
impl_primitives!([f64, f32], push_number, to_number);

impl Push for bool {
    fn push(&self, state: &mut State) -> Result<i32> {
        state.push_boolean(*self);
        Ok(1)
    }
}

impl Pull for bool {
    fn pull(state: &State, index: i32) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(state.to_boolean(index))
    }
}

impl Push for &[u8] {
    fn push(&self, state: &mut State) -> Result<i32> {
        state.push_string(*self)?;
        Ok(1)
    }
}

impl Pull for Vec<u8> {
    fn pull(state: &State, index: i32) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(state.as_bytes(index).to_vec())
    }
}

impl Pull for String {
    fn pull(state: &State, index: i32) -> Result<Self>
    where
        Self: Sized,
    {
        let vec = state.as_bytes(index).to_vec();
        let s = String::from_utf8(vec)?;
        Ok(s)
    }
}

macro_rules! impl_tuples {
    ($len:tt, $($idx:tt $T:ident)+) => {
        impl<$($T: Push),+> Push for ($($T,)+) {
            fn push(&self, state: &mut State) -> Result<i32> {
                let mut n = 0;
                $(n += self.$idx.push(&mut *state)?;)+
                Ok(n)
            }
        }

        impl<$($T: Pull),+> Pull for ($($T,)+) {
            fn size() -> i32 {
                0 $(+ $T::size())*
            }

            fn pull(state: &State, index: i32) -> Result<Self>
            where
                    Self: Sized {
                Ok(($($T::pull(&*state, index - ($len - 1) + $idx)?,)+))
            }
        }
    };
}

impl_tuples! { 1, 0 A }
impl_tuples! { 2, 0 A 1 B }
impl_tuples! { 3, 0 A 1 B 2 C}
impl_tuples! { 4, 0 A 1 B 2 C 3 D}
impl_tuples! { 5, 0 A 1 B 2 C 3 D 4 E}
impl_tuples! { 6, 0 A 1 B 2 C 3 D 4 E 5 F}
impl_tuples! { 7, 0 A 1 B 2 C 3 D 4 E 5 F 6 G}
impl_tuples! { 8, 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H}
impl_tuples! { 9, 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I}
impl_tuples! { 10, 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J}
impl_tuples! { 11, 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J 10 K}
impl_tuples! { 12, 0 A 1 B 2 C 3 D 4 E 5 F 6 G 7 H 8 I 9 J 10 K 11 L}

/// A Lua state.
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
    ptr: NonNull<ffi::lua_State>,
    droppable: bool,
}

unsafe impl Send for State {}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State({:p})", self.ptr)
    }
}

impl fmt::Pointer for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

impl State {
    /// Creates a new `State` without a memory limit.
    ///
    /// # Panics
    ///
    /// Panics when the ptr is non-null.
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
    /// # Panics
    ///
    /// Panics when the ptr is non-null.
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
        // initialize Lua user data
        let ud = Box::into_raw(box alloc::MemoryInfo::new(limit));

        // initialize raw Lua state
        let ptr = unsafe { ffi::lua_newstate(alloc::alloc, ud as _) };
        debug!("{:p} new state", ptr);

        Self::from_ptr(ptr, true)
    }

    /// Gets a mutable pointer to the Lua state pointer.
    fn as_ptr(&self) -> *mut ffi::lua_State {
        self.ptr.as_ptr()
    }

    /// Constructs a new `State`.
    ///
    /// # Panics
    ///
    /// Panics when the ptr is non-null.
    pub fn from_ptr(ptr: *mut ffi::lua_State, droppable: bool) -> Self {
        // panic when pointer is null, that is when not enough memory could be
        // allocated for the Lua state.
        if ptr.is_null() {
            panic!("failed to allocate enough memory for a Lua state");
        }

        // SAFETY: This unsafety is ok becuase the pointer is already checked and
        // is non-null.
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        Self { ptr, droppable }
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
    ///
    /// Always removes the function and its arguments from the stack.
    pub fn pcall(&mut self, nargs: i32, nresults: i32, msgh: i32) -> Result<()> {
        let code = unsafe { ffi::lua_pcall(self.as_ptr(), nargs, nresults, msgh) };
        self.handle_result(code, ())
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

    /// Pushes the C function on the call and call it in protected mode.
    pub fn call_secure(
        &mut self,
        nargs: i32,
        nresults: i32,
        msgh: i32,
        function: CFunction,
    ) -> Result<()> {
        self.push_cfunction(function);
        self.pcall(nargs, nresults, msgh)
    }

    /// Raises a Lua error, using the value on the top of the stack as the error object.
    ///
    /// This underlying C function does a long jump, and therefore never returns
    pub fn raise_error<E>(&mut self, error: E) -> !
    where
        E: Into<Box<dyn std::error::Error>>,
    {
        let error = error.into().to_string();
        if let Err(e) = self.push_string(error) {
            error!("failed to push error string to the stack, {}", e);
        }
        unsafe { ffi::lua_error(self.as_ptr()) }
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

    /// Pushes a new C closure onto the stack. This function receives a pointer to a C function and
    /// pushes onto the stack a Lua value of type function that, when called, invokes the corresponding
    /// C function. The parameter n tells how many upvalues this function will have (see [`§4.2`]).
    ///
    /// Any function to be callable by Lua must follow the correct protocol to receive its parameters
    /// and return its results (see [`CFunction`]).
    ///
    /// When a C function is created, it is possible to associate some values with it, the so called
    /// upvalues; these upvalues are then accessible to the function whenever it is called. This
    /// association is called a C closure (see [`§4.2`]). To create a C closure, first the initial
    /// values for its upvalues must be pushed onto the stack. (When there are multiple upvalues,
    /// the first value is pushed first.) Then lua_pushcclosure is called to create and push the C
    /// function onto the stack, with the argument n telling how many values will be associated with
    /// the function. lua_pushcclosure also pops these values from the stack.
    ///
    /// The maximum value for `n` is 255.
    ///
    /// When `n` is zero, this function creates a light C function, which is just a pointer to the C
    /// function. In that case, it never raises a memory error.
    ///
    /// [`§4.2`]: https://www.lua.org/manual/5.4/manual.html#4.2
    pub fn push_cclosure(&mut self, function: CFunction, n: i32) {
        unsafe { ffi::lua_pushcclosure(self.as_ptr(), function, n) }
    }

    /// Pushes a C function onto the stack.
    pub fn push_cfunction(&mut self, function: CFunction) {
        unsafe { ffi::lua_pushcfunction(self.as_ptr(), function) }
    }

    /// Pushes a float with value `t` onto the stack.
    pub fn push_number<T: Into<f64>>(&mut self, t: T) {
        let n = t.into();
        unsafe { ffi::lua_pushnumber(self.as_ptr(), n) }
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
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "unexpected NULL while pushing string",
                ));
            }
            let len = libc::strlen(cs);
            let data = cs as *const u8;
            std::slice::from_raw_parts(data, len)
        };
        Ok(s)
    }

    /// Pushes the value `p` onto the stack and returns the number of slots used.
    pub fn push<T: Push>(&mut self, t: T) -> Result<i32> {
        t.push(self)
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
    pub fn to_integer<T: num_traits::NumCast>(&self, index: i32) -> Option<T> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tointegerx(self.as_ptr(), index, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            num_traits::cast(n)
        }
    }

    /// Converts the Lua value at the given `index` to a float.
    pub fn to_number<T: num_traits::NumCast>(&self, index: i32) -> Option<T> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tonumberx(self.as_ptr(), index, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            num_traits::cast(n)
        }
    }

    /// If the value at the given `index` is a full userdata, returns its memory-block address. If
    /// the value is a light userdata, returns its value (a pointer). Otherwise, returns NULL.
    pub fn to_userdata(&self, index: i32) -> *mut c_void {
        unsafe { ffi::lua_touserdata(self.as_ptr(), index) }
    }

    /// Converts the Lua value at the given `index` to a C string.
    pub fn as_c_str<'a>(&'a self, index: i32) -> &'a CStr {
        unsafe { CStr::from_ptr(ffi::lua_tostring(self.as_ptr(), index)) }
    }

    /// Returns the [`Info`] of the stack element at the given `index`.
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

    /// Accepts any `index`, or 0, and sets the stack top to this `index`. If the new top is greater
    /// than the old one, then the new elements are filled with **nil**. If `index` is 0, then all
    /// stack elements are removed.
    ///
    /// This function can run arbitrary code when removing an index marked as to-be-closed from the
    /// stack.
    pub fn set_top(&mut self, index: i32) {
        unsafe { ffi::lua_settop(self.as_ptr(), index) }
    }

    /// Pops n elements from the stack.
    ///
    /// This can run arbitrary code when removing an index marked as to-be-closed from the stack.
    pub fn pop(&mut self, n: i32) {
        unsafe { ffi::lua_pop(self.as_ptr(), n) }
    }

    /// Pushes a copy of the element at the given `index` onto the stack.
    pub fn push_value(&mut self, index: i32) {
        unsafe { ffi::lua_pushvalue(self.as_ptr(), index) }
    }

    /// Rotates the stack elements between the valid index idx and the top of the stack.
    ///
    /// The elements are rotated `n` positions in the direction of the top, for a positive `n`, or
    /// `-n` positions in the direction of the bottom, for a negative `n`. The absolute value of `n`
    /// must not be greater than the size of the slice being rotated.
    ///
    /// ## Pseudo-index support
    ///
    /// This function cannot be called with a pseudo-index, because a pseudo-index is not an actual
    /// stack position.
    pub fn rotate(&mut self, index: i32, n: i32) {
        unsafe { ffi::lua_rotate(self.as_ptr(), index, n) }
    }

    /// Removes the element at the given valid `index`, shifting down the elements above this `index`
    /// to fill the gap.
    ///
    /// ## Pseudo-index support
    ///
    /// This function cannot be called with a pseudo-index, because a pseudo-index is not an actual
    /// stack position.
    pub fn remove(&mut self, index: i32) {
        unsafe { ffi::lua_remove(self.as_ptr(), index) }
    }

    /// Moves the top element into the given valid `index`, shifting up the elements above this
    /// `index` to open space.
    ///
    /// ## Pseudo-index support
    ///
    /// This function cannot be called with a pseudo-index, because a pseudo-index is not an actual
    /// stack position.
    pub fn insert(&mut self, index: i32) {
        unsafe { ffi::lua_insert(self.as_ptr(), index) }
    }

    /// Moves the top element into the given valid `index` without shifting any element (therefore
    /// replacing the value at that given `index`), and then pops the top element.
    pub fn replace(&mut self, index: i32) {
        unsafe { ffi::lua_replace(self.as_ptr(), index) }
    }

    /// Copies the element at index `fromidx` into the valid index `toidx`, replacing the value at
    /// that position. Values at other positions are not affected.
    pub fn copy(&mut self, fromidx: i32, toidx: i32) {
        unsafe { ffi::lua_copy(self.as_ptr(), fromidx, toidx) }
    }

    /// Returns an iterator over the stack (from bottom to top).
    pub fn iter(&self) -> Iter<'_> {
        let top = self.top();
        Iter {
            state: self,
            n: 1,
            top,
        }
    }

    /// Creates an iterator which dumps the values on a Lua stack from bottom to top.
    pub fn dump(&self) -> Dump<'_> {
        Dump { iter: self.iter() }
    }

    /// Pushes onto the stack the value of the global name. Returns the type of that value.
    pub fn get_global<T: Into<Vec<u8>>>(&mut self, name: T) -> Result<i32> {
        let name = CString::new(name)?;
        Ok(unsafe { ffi::lua_getglobal(self.as_ptr(), name.as_ptr()) })
    }

    pub fn set_global<T: Into<Vec<u8>>>(&mut self, name: T) -> Result<()> {
        let name = CString::new(name)?;
        Ok(unsafe { ffi::lua_setglobal(self.as_ptr(), name.as_ptr()) })
    }

    /// Pushes onto the stack the value `t[k]`, where `t` is the value at the given index and `k` is
    /// the value on the top of the stack.
    ///
    /// This methods pops the key from the stack, pushing the resulting value in its place. As in
    /// Lua, this function may trigger a metamethod for the "index" event (see [`§2.4`]).
    ///
    /// Returns the type of the pushed value.
    ///
    /// [`§2.4`]: https://www.lua.org/manual/5.4/manual.html#2.4
    pub fn get_table(&mut self, index: i32) -> i32 {
        unsafe { ffi::lua_gettable(self.as_ptr(), index) }
    }

    /// Pushes onto the stack the value `t[k]`, where `t` is the value at the given `index`. As in
    /// Lua, this function may trigger a metamethod for the "index" event (see [`§2.4`]).
    ///
    /// Returns the type of the pushed value.
    ///
    /// [`§2.4`]: https://www.lua.org/manual/5.4/manual.html#2.4
    pub fn get_field<T: Into<Vec<u8>>>(&mut self, index: i32, key: T) -> Result<i32> {
        let key = CString::new(key)?;
        Ok(unsafe { ffi::lua_getfield(self.as_ptr(), index, key.as_ptr()) })
    }

    /// Does the equivalent to `t[k] = v`, where `t` is the value at the given index, `v` is the
    /// value on the top of the stack, and `k` is the value just below the top.
    ///
    /// This function pops both the key and the value from the stack. As in Lua, this function may
    /// trigger a metamethod for the "newindex" event (see [`§2.4`]).
    ///
    /// [`§2.4`]: https://www.lua.org/manual/5.4/manual.html#2.4
    pub fn set_table(&mut self, index: i32) {
        unsafe { ffi::lua_settable(self.as_ptr(), index) }
    }

    /// Does the equivalent to `t[k] = v`, where `t` is the value at the given `index` and `v` is
    /// the value on the top of the stack.
    ///
    /// This function pops the value from the stack. As in Lua, this function may trigger a
    /// metamethod for the "newindex" event (see [`§2.4`]).
    ///
    /// [`§2.4`]: https://www.lua.org/manual/5.4/manual.html#2.4
    pub fn set_field<T: Into<Vec<u8>>>(&mut self, index: i32, key: T) -> Result<()> {
        let key = CString::new(key)?;
        Ok(unsafe { ffi::lua_setfield(self.as_ptr(), index, key.as_ptr()) })
    }

    /// Creates a new empty table and pushes it onto the stack.
    pub fn new_table(&mut self) {
        unsafe { ffi::lua_newtable(self.as_ptr()) }
    }

    /// Creates a new empty table and pushes it onto the stack. Parameter `narr` is a hint for how
    /// many elements the table will have as a sequence; parameter `nrec` is a hint for how many
    /// other elements the table will have. Lua may use these hints to preallocate memory for the
    /// new table. This preallocation may help performance when you know in advance how many elements
    /// the table will have. Otherwise you can use [`.new_table()`](State::new_table).
    pub fn create_table(&mut self, narr: i32, nrec: i32) {
        unsafe { ffi::lua_createtable(self.as_ptr(), narr, nrec) }
    }

    /// This function creates and pushes on the stack a new full userdata, with `nuvalue` associated
    /// Lua values, called user values, plus an associated block of raw memory with `size` bytes.
    /// (The user values can be set and read with the functions lua_setiuservalue and lua_getiuservalue.)
    ///
    /// The function returns the address of the block of memory. Lua ensures that this address is
    /// valid as long as the corresponding userdata is alive (see [`§2.5`]). Moreover, if the
    /// userdata is marked for finalization (see [`§2.5.3`]), its address is valid at least until the
    /// call to its finalizer.
    ///
    /// [`§2.5`]: https://www.lua.org/manual/5.4/manual.html#2.5
    /// [`§2.5.3`]: https://www.lua.org/manual/5.4/manual.html#2.5.3
    pub fn new_userdata(&mut self, size: usize, nuvalue: i32) -> *mut c_void {
        unsafe { ffi::lua_newuserdatauv(self.as_ptr(), size, nuvalue) }
    }

    /// Returns the pseudo-index that represents the `i`-th upvalue of the running function (see
    /// [`§4.2`]). `i` must be in the range [1,256].
    ///
    /// [`§4.2`]: https://www.lua.org/manual/5.4/manual.html#2.5
    pub fn upvalue_index(&self, i: i32) -> i32 {
        ffi::lua_upvalueindex(i)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if !self.droppable {
            return;
        }
        debug!("{:p} drop state", self.ptr);
        unsafe {
            // SAFETY: This unsafety is ok becuase while this `State` is alive
            // we're guaranteed that the inner pointer was not freed before.
            ffi::lua_close(self.as_ptr())
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
        self.iter.next().map(|(idx, _, state)| state.info(idx))
    }
}

/// A guard on the Lua stack size, that is it sets an expected low watermark for the stack size at
/// creation and when it gets out of scope will pop the elements above this low watermark. When the
/// stack size is below the low watermark it logs an error and terminates the process in an abnormal
/// fashion.
#[derive(Debug)]
pub struct StackGuard<'a> {
    mark: i32,
    state: &'a mut State,
}

impl<'a> StackGuard<'a> {
    /// Creates a new `StackGuard` with the stack size as low watermark.
    pub fn new(state: &'a mut State) -> Self {
        let top = state.top();
        Self::with_mark(top, state)
    }

    /// Creates a new `StackGuard` for a specified low watermark
    pub fn with_mark(mark: i32, state: &'a mut State) -> Self {
        Self { state, mark }
    }
}

impl<'a> Drop for StackGuard<'a> {
    fn drop(&mut self) {
        let top = self.state.top();
        if self.mark < top {
            debug!("[StackGuard] popping {} element(s)", top - self.mark);
            self.state.set_top(self.mark);
        } else if self.mark > top {
            error!(
                "[StackGuard] size ({}) under low watermark ({})",
                top, self.mark
            );
            std::process::abort()
        }
    }
}

impl<'a> Deref for StackGuard<'a> {
    type Target = State;
    fn deref(&self) -> &Self::Target {
        &(*self.state)
    }
}

impl<'a> DerefMut for StackGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<'a> AsRef<State> for StackGuard<'a> {
    fn as_ref(&self) -> &State {
        &(*self.state)
    }
}

impl<'a> AsMut<State> for StackGuard<'a> {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

/// A function defined in Lua.
pub struct Function<'a, Args, Output> {
    state: &'a mut State,
    name: &'a str,
    _marker: PhantomData<(Args, Output)>,
}

impl<'a, Args, Output> Function<'a, Args, Output> {
    /// Creates a new `Function` for given state and global name.
    pub fn new(state: &'a mut State, name: &'a str) -> Self {
        Self {
            state,
            name,
            _marker: PhantomData,
        }
    }
}

impl<'a, Args: Push, Output: Pull> FnOnce<Args> for Function<'a, Args, Output> {
    type Output = Result<Output>;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        let mut state = StackGuard::new(self.state);

        // push functions and arguments
        state.get_global(self.name)?; // push function
        let nargs = args.push(&mut state)?;

        // do the call (2 arguments, 1 result)
        state.pcall(nargs, ffi::LUA_MULTRET, 0)?;

        // retrieve the result(s)
        Output::pull(&state, -1)
    }
}

/// A Rust function wrapper.
pub struct RustFunction<F, Args, Output> {
    func: F,
    _marker: PhantomData<(Args, Output)>,
}

impl<F, Args, Output> RustFunction<F, Args, Output> {
    /// Creates a new `RustFunction` wrapping specified func.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _marker: PhantomData,
        }
    }
}

impl<F, Args, Output> Push for RustFunction<F, Args, Output>
where
    F: Fn(Args) -> Result<Output>,
    Args: Pull,
    Output: Push,
{
    fn push(&self, state: &mut State) -> Result<i32> {
        unsafe {
            let wrapped = wrapper::<Output, Args, F>;

            let ud = state.new_userdata(mem::size_of::<F>(), 1);
            let func: &mut F = mem::transmute(ud);
            ptr::copy(&self.func, func, 1);

            state.push_cclosure(wrapped, 1);
        }
        Ok(1)
    }
}

unsafe extern "C" fn wrapper<Output, Args, F>(ptr: *mut ffi::lua_State) -> i32
where
    F: Fn(Args) -> Result<Output>,
    Args: Pull,
    Output: Push,
{
    let mut state = State::from_ptr(ptr, false);

    let idx = state.upvalue_index(1);
    let func: &mut F = mem::transmute(state.to_userdata(idx));

    let ret = Args::pop(&mut state)
        .and_then(|args| func(args))
        .and_then(|output| output.push(&mut state));

    match ret {
        Ok(n) => {
            debug!("successfully called Lua function, {} element(s) pushed", n);
            n // number of results
        }
        Err(error) => {
            error!("failure calling Lua function, {}", error);
            if let Err(e) = state.push_string(error.to_string()) {
                error!("failed to push error string, {}", e);
            }
            ffi::LUA_ERRRUN
        }
    }
}
