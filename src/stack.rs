use std::{borrow::Borrow, convert::TryFrom, io, ffi::CString, fmt, ops::Deref};

use serde::{Serialize, Deserialize};

use super::{ffi, state::State, de, ser};

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
    /// use lua::State;
    ///
    /// let stack = State::default().as_stack();
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
    /// use lua::State;
    ///
    /// let stack = State::default().as_stack();
    /// assert_eq!(0, stack.len());
    ///
    /// stack.to_owned().push_slice(&[1, 2, 3]);
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
    /// use lua::State;
    ///
    /// let stack = State::default().as_stack();
    /// assert!(stack.is_empty());
    ///
    /// stack.to_owned().push(1);
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
    pub fn set_top(&mut self, index: i32) {
        trace!("set_top() index = {}", index);
        unsafe { ffi::lua_settop(self.as_ptr(), index) }
    }

    pub fn resize(&mut self, _new_len: usize) {

    }

    /// Returns the type of the value in the given valid `index`, or [`LUA_TNONE`] for a non-valid
    /// but acceptable index.
    pub fn value_type(&self, index: i32) -> i32 {
        unsafe { ffi::lua_type(self.as_ptr(), index) }
    }

    /// Returns the element on the top of the stack.
    pub fn get<'de, T>(&'de self) -> Result<T, de::Error>
    where
        T: Deserialize<'de>,
    {
        let mut deserializer = de::Deserializer::new(self);
        T::deserialize(&mut deserializer)
    }

    /// Pops and returns the element on the top of the stack.
    /// 
    /// This function can run arbitrary code when removing an index marked as to-be-closed from the
    /// stack.
    pub fn pop<'de, T>(&'de self) -> Result<T, de::Error>
    where
        T: Deserialize<'de>,
    {
        let t = self.get();
        unsafe { ffi::lua_pop(self.as_ptr(), 1) };
        t
    }

    /// Loads a reader as a Lua chunk, without running it. If there are no errors, it pushes the
    /// compiled chunk as a Lua function on top of the stack. Otherwise, it returns an error message.
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

    pub fn call(&mut self, nargs: i32, nresults: i32, msgh: i32) -> io::Result<()> {
        trace!("State::call() nargs = {}, nresults = {}, msgh = {}", nargs, nresults, msgh);

        let code = unsafe { ffi::lua_pcall(self.as_ptr(), nargs, nresults, msgh) };

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
    /// Serializes the `value` and sets it as the new value of the global `name`.
    pub fn insert<T>(&mut self, name: &str, value: &T) -> Result<(), ser::Error>
    where
        T: Serialize + fmt::Debug,
    {
        trace!("Globals::set() name = {:?}, value = {:?}", name, value);

        value.serialize(&mut self.stack)?;

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
    pub fn get<'de, T>(&'de self, name: &str) -> Result<T, de::Error>
    where
        T: Deserialize<'de>,
    {
        trace!("Globals::get() name = {:?}", name);

        unsafe {
            let name = CString::new(name).map_err(|error| {
                de::Error::new(error.to_string())
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
