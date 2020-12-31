use super::{ffi, stack::Stack};

/// An access to a Lua table.
///
/// The type table implements associative arrays, that is, arrays that can have as indices not only
/// numbers, but any Lua value except nil and NaN. (Not a Number is a special floating-point value
/// used by the IEEE 754 standard to represent undefined numerical results, such as 0/0.) Tables can
/// be heterogeneous; that is, they can contain values of all types (except nil). Any key associated
/// to the value nil is not considered part of the table. Conversely, any key that is not part of a
/// table has an associated value nil.
pub struct Table {
    stack: Stack,
}

/// Enumeration of possible keys of a Lua table.
/// 
/// It is used by [`get()`](Table::get) and [`set()`](Table::set) methods on `Table`.
pub enum Key<'a> {
    /// A byte slice based key.
    /// 
    /// ## Setting values
    /// 
    /// Does the equivalent to `t[k] = v`, where `t` is the value at the table index and `v` is the
    /// value on the top of the stack.
    /// 
    /// This pops the value from the stack.
    /// 
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { 
    /// let mut stack = Stack::new();
    ///
    /// // create a new empty table and push it on the stack
    /// let table = Table::new(&mut stack);
    /// 
    /// // push the value onto the stack
    /// table.as_mut().push(1989_u16)?;
    /// 
    /// // set the value into the table
    /// //table.set(-2, "key");
    /// 
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// As in Lua, this may trigger a metamethod for the `newindex` event.
    /// 
    /// ## Getting values
    ///
    /// Pushes onto the stack the value `t[key]`. where `t` is the table at given `index`.
    /// 
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { 
    /// let mut stack = Stack::new();
    ///
    /// // create a new empty table and push it on the stack
    /// let table = Table::new(&mut stack);
    /// 
    /// // push the value onto the stack
    /// table.as_mut().push(1989_u16)?;
    /// 
    /// // pops the value from the table and store it into the table
    /// table.set(-2, "key");
    /// 
    /// // pushes the value in the table with key "key" onto the stack
    /// table.get(-1, "key");
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// As in Lua, uaing this variant may trigger a metamethod for the `index` event.
    Field(&'a [u8]),
    /// An integer key.
    /// 
    /// ## Setting values
    /// 
    /// Does the equivalent to `t[n] = v`, where `t` is the table at the given index and `v` is the
    /// value on the top of the stack.
    /// 
    /// This pops the value from the stack.
    /// 
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    /// 
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { 
    /// let mut stack = Stack::new();
    ///
    /// // create a new empty table and push it on the stack
    /// let table = Table::new(&mut stack);
    /// 
    /// // push the value onto the stack
    /// table.as_mut().push(1989_u16)?;
    /// 
    /// // set the value into the table
    /// //table.set(-2, 1);
    /// 
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// As in Lua, this may trigger a metamethod for the `newindex` event.
    /// 
    /// ## Getting values
    ///
    /// Pushes onto the stack the value `t[n]`. where `t` is the table at given `index`.
    /// 
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> { 
    /// let mut stack = Stack::new();
    ///
    /// // create a new empty table and push it on the stack
    /// let table = Table::new(&mut stack);
    /// 
    /// // push the value onto the stack
    /// table.as_mut().push(1989_u16)?;
    /// 
    /// // pops the value from the table and store it into the table
    /// table.set(-2, 1);
    /// 
    /// // pushes the value in the table with index `1` onto the stack
    /// table.get(-1, 1);
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// As in Lua, uaing this variant may trigger a metamethod for the `index` event.
    Index(i64),
    RawIndex(i64),
    RawTable,
    Table,
}

impl From<i32> for Key<'_> {
    fn from(i: i32) -> Self {
        Key::Index(i.into())
    }
}

impl From<i64> for Key<'_> {
    fn from(i: i64) -> Self {
        Key::Index(i)
    }
}

impl<'a> From<&'a str> for Key<'a> {
    fn from(s: &'a str) -> Self {
        Key::Field(s.as_bytes())
    }
}

impl Table {
    /// Creates a new empty table and pushes it onto the stack. It is equivalent to
    /// [`.create(&stack, 0, 0)`](Table::create).
    /// 
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    ///
    /// let mut stack = Stack::new();
    /// let t = Table::new(&mut stack);
    /// ```
    pub fn new<S: AsMut<Stack>>(stack: &mut S) -> &mut Self {
        Self::create(stack, 0, 0)
    }

    /// Creates a new empty table and pushes it onto the stack.
    ///
    /// Parameter `narr` is a hint for how many elements the table will have as a sequence; parameter
    /// `nrec` is a hint for how many other elements the table will have. Lua may use these hints to
    /// preallocate memory for the new table. This preallocation may help performance when you know
    /// in advance how many elements the table will have.
    ///
    /// Otherwise you can use [`.new(&stack)`](Table::new).
    /// 
    /// # Examples
    ///
    /// ```
    /// # extern crate lua;
    /// use lua::{Stack, table::Table};
    ///
    /// let mut stack = Stack::new();
    /// let t = Table::new(&mut stack);
    /// ```
    pub fn create<S: AsMut<Stack>>(stack: &mut S, narr: i32, nrec: i32) -> &mut Self {
        unsafe {
            let state = stack.as_mut().as_ptr();

            ffi::lua_createtable(state, narr, nrec);
        }

        unsafe { &mut *(stack.as_mut() as *mut Stack as *mut Table) }
    }

    /// Pushes onto the stack the value `t[key]`. where `t` is the value at given `index`.
    ///
    /// Returns the type of the pushed value.
    ///
    /// The type of [`Key`] impacts the behaviour. See its documentation for more info.
    pub fn get<'a, K>(&self, index: i32, key: K) -> i32 
    where
        K: Into<Key<'a>>,
    {
        unsafe {
            let state = self.stack.as_ptr();

            match key.into() {
                Key::Field(k) => {
                    ffi::lua_getfield(state, index, k.as_ptr() as *const i8)
                }
                Key::Index(i) => {
                    ffi::lua_geti(state, index, i)
                }
                Key::RawIndex(n) => {
                    ffi::lua_rawgeti(state, index, n)
                }
                Key::RawTable => {
                    ffi::lua_rawget(state, index)
                }
                Key::Table => {
                    ffi::lua_gettable(state, index)
                }
            }
        }
    }

    /// Does the equivalent to `t[key] = v`, where `t` is the value at the given index and `v` is
    /// the value on the top of the stack.
    ///
    /// The type of [`Key`] impacts the behaviour. See its documentation for more info.
    pub fn set<'a, K>(&self, index: i32, key: K) 
    where
        K: Into<Key<'a>>,
    {
        unsafe {
            let state = self.stack.as_ptr();

            match key.into() {
                Key::Field(k) => {
                    ffi::lua_setfield(state, index, k.as_ptr() as *const i8)
                }
                Key::Index(i) => {
                    ffi::lua_seti(state, index, i)
                }
                Key::RawIndex(n) => {
                    ffi::lua_rawseti(state, index, n)
                }
                Key::RawTable => {
                    ffi::lua_rawset(state, index)
                }
                Key::Table => {
                    ffi::lua_settable(state, index)
                }
            }
        }
    }
}

impl AsRef<Stack> for Table {
    fn as_ref(&self) -> &Stack {
        &self.stack
    }
}

impl AsMut<Stack> for Table {
    fn as_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }
}

impl From<Stack> for Table {
    fn from(stack: Stack) -> Self {
        Self { stack }
    }
}

impl From<Table> for Stack {
    fn from(table: Table) -> Self {
        table.stack
    }
}