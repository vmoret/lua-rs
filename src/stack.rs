use std::{borrow::Borrow, convert::TryFrom, io, ffi::CString, fmt, ops::Deref};

use serde::{Serialize, Deserialize};

use super::{ffi, state::State, de::Deserializer, error::{Error, Result}};

/// A type that can be pushed onto a Lua stack.
///
/// Out of the box this crate provides `Push` implementations for all types that
/// implement [`Serialize`].
///
/// # Examples
///
/// ```
/// # extern crate lua;
/// use lua::Stack;
/// use crate::lua::Push;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut stack = Stack::new();
/// 1989_i32.push(&mut stack)?;
/// assert_eq!(1, stack.len());
/// # Ok(())
/// # }
/// ```
pub trait Push {
    /// Pushes this value onto the given stack and returns the number of slots
    /// used (typically that will be 1).
    fn push(&self, stack: &mut Stack) -> Result<i32>;
}


impl<T: Serialize> Push for T {
    fn push(&self, stack: &mut Stack) -> Result<i32> {
        self.serialize(stack)
    }
}

/// A Lua stack.
#[derive(Debug, Clone)]
pub struct Stack {
    state: State,
}

/// Lua chunk mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// Text only chunk.
    Text,
    /// Binary only chunk.
    Binary,
    /// Undefined chunk, can be binary or text.
    Undefined,
}

impl From<Mode> for &str {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Text => "t",
            Mode::Binary => "b",
            Mode::Undefined => "bt",
        }
    }
}

impl Stack {
    /// Constructs a new, empty `Stack`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let stack = Stack::new();
    /// ```
    pub fn new() -> Self {
        State::new().into()
    }

    /// Constructs a new, empty `Stack` with the specified memory limit.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let stack = Stack::with_limit(8 * 1_024);
    /// ```
    pub fn with_limit(limit: usize) -> Self {
        State::with_limit(limit).into()
    }

    /// Returns a reference to the underlying [`State`] in this stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let stack = Stack::new();
    /// let state = stack.as_state();
    /// ```
    pub fn as_state(&self) -> &State {
        &self.state
    }

    /// Returns a mutable reference to the underlying [`State`] in this stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// let state = stack.as_mut_state();
    /// ```
    pub fn as_mut_state(&mut self) -> &mut State {
        &mut self.state
    }

    /// Consumes the `Stack` into a [`State`].
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let stack = Stack::new();
    /// let state = stack.into_state();
    /// ```
    pub fn into_state(self) -> State {
        self.state
    }

    /// Gets a mutable pointer to the Lua state pointer.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut ffi::lua_State {
        self.state.as_ptr()
    }

    /// Returns a reference to the Lua globals.
    pub fn as_globals(&self) -> &Globals {
        Globals::new(self)
    }

    /// Ensures that the stack has space for at least `n` extra elements, that is, that you can
    /// safely push up to `n` values into it. It returns `false` if it cannot fulfill the request,
    /// either because it would cause the stack to be greater than a fixed maximum size (typically
    /// at least several thousand elements) or because it cannot allocate memory for the extra space.
    ///
    /// This function never shrinks the stack; if the stack already has space for the extra elements,
    /// it is left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push(1);
    /// assert!(!stack.is_empty());
    /// ```
    pub fn reserve(&mut self, n: i32) -> bool {
        unsafe { ffi::lua_checkstack(self.as_ptr(), n) != 0 }
    }

    /// Returns the index of the top element in the stack.
    ///
    /// Because indices start at 1, this result is equal to the number of elements in the stack; in
    /// particular, 0 means an empty stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert_eq!(0, stack.top());
    /// ```
    pub fn top(&self) -> i32 {
        unsafe { ffi::lua_gettop(self.as_ptr()) }
    }

    /// Returns the number of elements in the stack, also referred to as its 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert_eq!(0, stack.len());
    ///
    /// stack.push_slice(&[1, 2, 3]);
    /// assert_eq!(3, stack.len());
    /// ```
    pub fn len(&self) -> usize {
        // SAFETY: The unwrap is ok because `top()` is guaranteed to be zero or more and less than
        // i32::MAX which is guaranteed to be less than usize::MAX.
        usize::try_from(self.top()).unwrap()
    }

    /// Returns `true` if the stack contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push(1);
    /// assert!(!stack.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.top() == 0
    }

    /// Accepts any `index`, or 0, and sets the stack top to this `index`. If the new top is greater
    /// than the old one, then the new elements are filled with **nil**. If `index` is 0, then all
    /// stack elements are removed.
    ///
    /// This function can run arbitrary code when removing an index marked as to-be-closed from the
    /// stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push(1);
    /// assert!(!stack.is_empty());
    /// ```
    pub fn set_top(&mut self, index: i32) {
        trace!("set_top() index = {}", index);
        unsafe { ffi::lua_settop(self.as_ptr(), index) }
    }

    /// Resizes the `Stack` in-place so that `len` is equal to `new_len`.
    /// 
    /// If `new_len` is greater than `len`, the `Stack` is extended by the difference, with each
    /// additional slot filled with **nil**. If `new_len` is less than `len`, the `Stack` is simply
    /// truncated.
    ///
    /// # Panics
    ///
    /// Panics if the new size exceeds `i32::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.resize(10);
    /// assert_eq!(10, stack.len());
    /// ```
    pub fn resize(&mut self, new_len: usize) {
        let index = i32::try_from(new_len).unwrap();
        self.set_top(index)
    }

    /// Shortens the stack, keeping the first `len` elements and dropping the rest.
    ///
    /// If `len` is greater than the stack's current length, this has no effect.
    ///
    /// # Panics
    ///
    /// Panics if the new size exceeds `i32::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push_slice(&[1, 2, 3, 4])?;
    /// assert_eq!(4, stack.len());
    ///
    /// stack.truncate(2);
    /// assert_eq!(2, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn truncate(&mut self, len: usize) {
        let index = i32::try_from(len).unwrap();
        let top = self.top();
        if index < top {
            self.set_top(index)
        }
    }

    /// Pushes an element onto a stack.
    ///
    /// This method requires `T` to implement [`Push`], in order to be able to push the element.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push(1)?;
    /// assert_eq!(1, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn push<T: Push>(&mut self, value: T) -> Result<i32> {
        value.push(self)
    }

    /// Pushes a slice of elements onto a stack.
    ///
    /// This method requires `T` to implement [`Push`], in order to be able to push the element.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push_slice(&[1, 2, 3, 4])?;
    /// assert_eq!(4, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn push_slice<T: Push>(&mut self, data: &[T]) -> Result<i32> {
        let mut i = 0;
        for value in data {
            i += value.push(&mut *self)?;
        }
        Ok(i)
    }

    /// Returns the type of the value in the given valid `index`, or [`LUA_TNONE`] for a non-valid
    /// but acceptable index.
    pub fn value_type(&self, index: i32) -> i32 {
        unsafe { ffi::lua_type(self.as_ptr(), index) }
    }

    /// Returns the element on the top of the stack.
    ///
    /// This method requires `T` to implement [`Deserialize`], in order to be able to desserialize
    /// the element.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push_slice(&[1, 2, 3, 4])?;
    /// assert_eq!(4, stack.len());
    /// let v: i32 = stack.get()?;
    /// assert_eq!(4, v);
    /// assert_eq!(4, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<'de, T>(&'de self) -> Result<T>
    where
        T: Deserialize<'de>,
    {
        let mut deserializer = Deserializer::new(self);
        T::deserialize(&mut deserializer)
    }

    /// Pops and returns the element on the top of the stack.
    /// 
    /// This function can run arbitrary code when removing an index marked as to-be-closed from the
    /// stack.
    ///
    /// This method requires `T` to implement [`Deserialize`], in order to be able to desserialize
    /// the element.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.push_slice(&[1, 2, 3, 4])?;
    /// assert_eq!(4, stack.len());
    /// let v: i32 = stack.pop()?;
    /// assert_eq!(4, v);
    /// assert_eq!(3, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    // TODO(vimo) make this mutable when Globals is merged with GlobalsMut.
    pub fn pop<'de, T>(&'de self) -> Result<T>
    where
        T: Deserialize<'de>,
    {
        let t = self.get();
        unsafe { ffi::lua_pop(self.as_ptr(), 1) };
        t
    }

    /// Loads a reader as a Lua chunk, without running it. If there are no errors, it pushes the
    /// compiled chunk as a Lua function on top of the stack. Otherwise, it returns an error message.
    ///
    /// `name` is the chunk name, used for debug information and error messages.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// const CHUNK: &str = "-- define window size
    /// width = 200
    /// height = 300
    /// ";
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.load_buffer(&mut CHUNK.as_bytes(), "test", lua::Mode::Text)?;
    /// assert_eq!(1, stack.len());
    /// stack.call(0, None)?;
    /// assert_eq!(0, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_buffer<R: io::Read>(&mut self, reader: &mut R, name: &str, mode: Mode) -> io::Result<usize> {
        trace!("State::load_buffer() name = {:?}, mode = {:?}", name, mode);

        let mut buf = Vec::with_capacity(4 * 1_024);
        let len = reader.read_to_end(&mut buf)?;

        let mode: &str = mode.into();
        let code = unsafe { ffi::luaL_loadbufferx(self.as_ptr(), buf.as_ptr() as _, buf.len(), name.as_ptr() as _, mode.as_ptr() as _) };

        if code == ffi::LUA_OK {
            Ok(len)
        } else {
            let error: &str = self.pop().map_err(|error| {
                io::Error::new(io::ErrorKind::InvalidData, error)
            })?;
            Err(io::Error::new(io::ErrorKind::InvalidData, error))
        }
    }

    /// Calls a function.
    ///
    /// Like regular Lua calls, this respects the `__call` metamethod. So, here the word "function"
    /// means any callable value.
    ///
    /// To do a call you must use the following protocol: 
    ///
    /// - the function to be called is pushed onto the stack
    /// - the arguments to the call are pushed in direct order; that is, the first argument is
    ///   pushed first.
    /// - you call [`.call()`](Stack::call); `nargs` is the number of arguments that you pushed onto
    ///   the stack.
    ///
    /// When the function returns, all arguments and the function value are popped and the call
    /// results are pushed onto the stack. The number of results is adjusted to `nresults`, unless
    /// `nresults` is `None`. In this case, all results from the function are pushed; Lua takes care
    /// that the returned values fit into the stack space, but it does not ensure any extra space in
    /// the stack. The function results are pushed onto the stack in direct order (the first result
    /// is pushed first), so that after the call the last result is on the top of the stack.
    /// 
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// const CHUNK: &str = "-- define window size
    /// width = 200
    /// height = 300
    /// ";
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    ///
    /// stack.load_buffer(&mut CHUNK.as_bytes(), "test", lua::Mode::Text)?;
    /// assert_eq!(1, stack.len());
    /// stack.call(0, None)?;
    /// assert_eq!(0, stack.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn call(&mut self, nargs: i32, nresults: Option<i32>) -> io::Result<()> {
        trace!("State::call() nargs = {}, nresults = {:?}", nargs, nresults);

        // From the manual (https://www.lua.org/manual/5.4/manual.html#lua_call)
        //
        // The number of results is adjusted to `nresults`, unless `nresults` is
        // `LUA_MULTRET`. In this case, all results from the function are pushed;
        // Lua takes care that the returned values fit into the stack space, but
        // it does not ensure any extra space in the stack. The function results
        // are pushed onto the stack in direct order (the first result is pushed
        // first), so that after the call the last result is on the top of the
        // stack.
        //
        // In our implementation we don't expose `LUA_MULTRET`. We opted to make
        // `nresults` an optional value, where `None` is translated to 
        // `LUA_MULTRET`.
        let nresults = nresults.unwrap_or(ffi::LUA_MULTRET);

        // From the manual (https://www.lua.org/manual/5.4/manual.html#lua_pcall):
        //
        // If msgh is 0, then the error object returned on the stack is exactly 
        // the original error object. Otherwise, msgh is the stack index of a 
        // message handler. (This index cannot be a pseudo-index.) In case of 
        // runtime errors, this handler will be called with the error object and
        // its return value will be the object returned on the stack by 
        // `lua_pcall`.
        //
        // Typically, the message handler is used to add more debug information
        // to the error object, such as a stack traceback. Such information
        // cannot be gathered after the return of `lua_pcall`, since by then the
        // stack has unwound.
        //
        // We opt for the simple approach, to get the error object returned on
        // the stack.
        let msgh = 0;

        let code = unsafe { ffi::lua_pcall(self.as_ptr(), nargs, nresults, msgh) };

        // The `lua_pcall` function returns one of the following status codes: 
        // LUA_OK, LUA_ERRRUN, LUA_ERRMEM, or LUA_ERRERR.
        if code == ffi::LUA_OK {
            Ok(())
        } else {
            let error: &str = self.pop().map_err(|error| {
                io::Error::new(io::ErrorKind::InvalidData, error)
            })?;
            Err(io::Error::new(io::ErrorKind::InvalidData, error))
        }
    }
}

impl From<State> for Stack {
    /// Converts a `State` into a `Stack`
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(state: State) -> Self {
        Self { state }
    }
}

impl From<Stack> for State {
    fn from(this: Stack) -> State {
        this.state
    }
}

impl Borrow<State> for Stack {
    fn borrow(&self) -> &State {
        &self.state
    }
}

impl AsRef<Stack> for Stack {
    fn as_ref(&self) -> &Stack {
        self
    }
}

/// A mutable access to the Lua global variables.
#[derive(Debug, Clone)]
pub struct GlobalsMut {
    stack: Stack,
}

impl GlobalsMut {
    /// Pushes the `value` and sets it as the new value of the global `name`.
    pub fn insert<T>(&mut self, name: &str, value: &T) -> Result<()>
    where
        T: Push + fmt::Debug,
    {
        trace!("Globals::set() name = {:?}, value = {:?}", name, value);

        value.push(&mut self.stack)?;

        // pops a value from the stack and set it as the new value of global name
        unsafe { ffi::lua_setglobal(self.stack.as_ptr(), name.as_ptr() as _) };

        Ok(())
    }
}

impl Deref for GlobalsMut {
    type Target = Globals;
    fn deref(&self) -> &Self::Target {
        Globals::new(&self.stack)
    }
}

impl From<Stack> for GlobalsMut {
    /// Converts a `Stack` into a `GlobalsMut`
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(stack: Stack) -> Self {
        Self { stack }
    }
}

impl Borrow<Globals> for GlobalsMut {
    fn borrow(&self) -> &Globals {
        self.deref()
    }
}

impl From<GlobalsMut> for Stack {
    fn from(this: GlobalsMut) -> Stack {
        this.stack
    }
}

/// An immutable access to the Lua global variables.
///
/// # Examples
///
/// ```
/// # extern crate lua;
/// use lua::Stack;
///
/// let stack = Stack::new();
/// let g = stack.as_globals();
/// ```
#[derive(Debug)]
pub struct Globals {
    stack: Stack,
}

impl Globals {
    /// Directly wraps a [`Stack`] as a `Globals` reference.
    /// 
    /// This is a cost-free conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::Stack;
    ///
    /// let stack = Stack::new();
    /// let g = stack.as_globals();
    /// ```
    pub fn new<S: AsRef<Stack>>(stack: &S) -> &Globals {
        unsafe { &*(stack.as_ref() as *const Stack as *const Globals) }
    }

    /// Returns the deserialized value of the global `name`.
    pub fn get<'de, T>(&'de self, name: &str) -> Result<T>
    where
        T: Deserialize<'de>,
    {
        trace!("Globals::get() name = {:?}", name);

        unsafe {
            let name = CString::new(name).map_err(|error| {
                Error::InvalidInput { name: "name".into(), error: error.to_string() }
            })?;
            let typ = ffi::lua_getglobal(self.stack.as_ptr(), name.as_ptr());
            debug!("Globals::get() name = {:?}: type = {}", name, typ);
        }

        self.stack.pop()
    }
}

impl ToOwned for Globals {
    type Owned = GlobalsMut;
    fn to_owned(&self) -> Self::Owned {
        GlobalsMut { stack: self.stack.clone() }
    }
}
