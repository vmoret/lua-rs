use std::{ffi::CString, fmt};

use crate::{
    error::{Error, Result},
    ffi,
    stack::{Pull, Push, Stack},
};

/// An access to the Lua global variables.
///
/// # Examples
///
/// ```
/// # extern crate lua;
/// use lua::{Stack, Globals};
///
/// const CHUNK: &str = "-- define window size
/// width = 200
/// height = 300
/// ";
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // load the chunk
/// let mut stack = Stack::new();
/// stack.load_buffer(&mut CHUNK.as_bytes(), "test", lua::Mode::Text)?;
/// stack.call(0, None)?;
///
/// // get the global values
/// let globals = Globals::new(&stack);
/// assert_eq!(200_u16, globals.get("width")?);
/// assert_eq!(300_u16, globals.get("height")?);
/// # Ok(())
/// # }
/// ```
pub struct Globals<S> {
    stack: S,
}

impl<S> Globals<S> {
    /// Creates a new globals wrapping the provided underlying stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, Globals};
    ///
    /// let stack = Stack::new();
    /// let globals = Globals::new(&stack);
    /// ```
    pub fn new(stack: S) -> Self {
        Self { stack }
    }
}

impl<S> Globals<S>
where
    S: AsMut<Stack>,
{
    /// Pushes the `value` and sets it as the new value of the global `name`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, Globals};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stack = Stack::new();
    /// let mut globals = Globals::new(&mut stack);
    ///
    /// globals.set("height", 200_u16)?;
    /// let height: u16 = globals.get("height")?;
    /// assert_eq!(200_u16, height);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set<K, V>(&mut self, name: K, value: V) -> Result<i32>
    where
        K: AsRef<str>,
        V: Push + fmt::Debug,
    {
        let name = name.as_ref();
        trace!("Globals::set() name = {:?}, value = {:?}", name, value);

        let n = value.push(self.stack.as_mut())?;

        // pops a value from the stack and set it as the new value of global name
        unsafe { ffi::lua_setglobal(self.stack.as_mut().as_ptr(), name.as_ptr() as _) };

        Ok(n)
    }
}

impl<S> Globals<S>
where
    S: AsRef<Stack>,
{
    /// Returns the deserialized value of the global `name`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, Globals};
    ///
    /// let stack = Stack::new();
    /// let globals = Globals::new(&stack);
    ///
    /// let height = globals.get::<&str, u16>("height");
    /// assert!(height.is_err());
    /// ```
    pub fn get<'a, K, V>(&'a self, name: K) -> Result<V>
    where
        K: AsRef<str>,
        V: Pull<'a>,
    {
        let name = name.as_ref();
        trace!("Globals::get() name = {:?}", name);

        let mut stack = self.stack.as_ref().clone();

        // push onto the stack the value of the global name.
        unsafe {
            let name = CString::new(name).map_err(|error| Error::InvalidInput {
                name: "name".into(),
                error: error.to_string(),
            })?;
            let typ = ffi::lua_getglobal(stack.as_ptr(), name.as_ptr());
            debug!("Globals::get() name = {:?}: type = {}", name, typ);
        }

        // get the value of the element on the top of the stack
        let v = self.stack.as_ref().get();

        // pop the element on the top of the stack
        stack.pop_unchecked(1);

        v
    }
}
