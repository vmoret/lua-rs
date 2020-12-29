use std::cell::Cell;
use crate::{ffi, State};

/// A reference to a Lua object.
#[derive(Debug)]
pub struct LRef {
    state: State,
    t: i32,
    lref: Cell<i32>,
}

unsafe impl Send for LRef {}

impl LRef {
    /// Creates and returns a reference, in the table at index `t`, for the object on the top of the
    /// stack (and pops the object).
    pub fn new(state: &State, t: i32) -> Self {
        // create the new reference for the object on the top of the stack (and
        // pop the object)
        let lref = unsafe { ffi::luaL_ref(state.as_ptr(), t) };
        Self::_new(state, t, lref)
    }

    /// Creates and returns a reference in the Lua registry, for the object on the top of the stack
    /// (and pops the object).
    pub fn register(state: &State) -> Self {
        Self::new(state, ffi::LUA_REGISTRYINDEX)
    }

    /// Creates and returns a reference, in the table at index `t` without an object.
    pub fn empty(state: &State, t: i32) -> Self {
        Self::_new(state, t, ffi::LUA_NOREF)
    }

    fn _new(state: &State, t: i32, lref: i32) -> Self {
        let this = Self { lref: Cell::new(lref), state: state.clone(), t };
        debug!("new {:?}", this);
        this
    }

    /// Replaces the actual value in the reference with a new reference for the object on the top of 
    /// the stack (and pops the object), returning the old value.
    pub fn replace(&self) -> i32 {
        debug!("replace {:?}", self);

        let state = self.state.as_ptr();
        let t = self.t;
        let lref = self.lref.get();
        
        unsafe {
            // remove reference
            self.unref();

            // create the new reference for the object on the top of the stack
            // (and pop the object)
            self.lref.set(ffi::luaL_ref(state, t));
        }

        lref
    }

    /// Takes the value out of the reference, leaving an empty reference.
    pub fn take(&self) -> i32 {
        debug!("take {:?}", self);

        let lref = self.lref.get();

        self._get();
        self.unref();

        // set the reference index to LUA_NOREF, which is guaranteed to be
        // different from any reference returned by `luaL_ref()`
        self.lref.set(ffi::LUA_NOREF);

        lref
    }

    /// Pushes onto the stack the value of the reference. The access is raw, that is, it does not
    /// use the `__index` metavalue.
    /// 
    /// Returns the type of the pushed value.
    pub fn get(&self) -> i32 {
        debug!("get {:?}", self);
        self._get()
    }

    fn _get(&self) -> i32 {
        unsafe { ffi::lua_rawgeti(self.state.as_ptr(), self.t, self.lref.get().into()) }
    }

    /// Release the reference, remove the entry from the table, so that the 
    /// referred object can be collected and free the inner referece to be used
    /// again
    fn unref(&self) {
        let lref = self.lref.get();
        if lref == ffi::LUA_NOREF {
            return;
        }
        unsafe { ffi::luaL_unref(self.state.as_ptr(), self.t, lref) }
    }
}

impl Drop for LRef {
    fn drop(&mut self) {
        debug!("drop {:?}", self);

        // release the reference, remove the entry from the table, so that the
        // referred object can be collected and free the inner referece to be
        // used again
        let lref = self.lref.get();
        unsafe { ffi::luaL_unref(self.state.as_ptr(), self.t, lref) }
    }
}
