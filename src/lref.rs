use std::cell::Cell;
use crate::{ffi, State};

#[derive(Debug)]
pub struct LRef {
    state: State,
    t: i32,
    lref: Cell<i32>,
}

unsafe impl Send for LRef {}

impl LRef {
    pub fn new(state: &State, t: i32) -> Self {
        let lref = unsafe { ffi::luaL_ref(state.as_ptr(), t) };
        Self::_new(state, t, lref)
    }

    pub fn register(state: &State) -> Self {
        Self::new(state, ffi::LUA_REGISTRYINDEX)
    }

    pub fn empty(state: &State, t: i32) -> Self {
        Self::_new(state, t, ffi::LUA_NOREF)
    }

    fn _new(state: &State, t: i32, lref: i32) -> Self {
        let this = Self { lref: Cell::new(lref), state: state.clone(), t };
        debug!("new {:?}", this);
        this
    }

    pub fn replace(&self) -> i32 {
        debug!("replace {:?}", self);

        let state = self.state.as_ptr();
        let t = self.t;
        let lref = self.lref.get();
        
        unsafe {
            // remove reference
            ffi::luaL_unref(state, t, lref);

            // create the new reference to the item on the top of the stack
            self.lref.set(ffi::luaL_ref(state, t));
        }

        lref
    }

    pub fn take(&self) -> i32 {
        debug!("take {:?}", self);

        let state = self.state.as_ptr();
        let t = self.t;
        let lref = self.lref.get();
        
        unsafe {
            // get the value and push it onto the stack
            ffi::lua_rawgeti(state, t, lref.into());

            // remove reference
            ffi::luaL_unref(state, t, lref);

            // create the new reference to the item on the top of the stack
            self.lref.set(ffi::LUA_NOREF);
        }

        lref
    }

    pub fn get(&self) -> i32 {
        debug!("get {:?}", self);
        unsafe { ffi::lua_rawgeti(self.state.as_ptr(), self.t, self.lref.get().into()) }
    }
}

impl Drop for LRef {
    fn drop(&mut self) {
        debug!("drop {:?}", self);
        let lref = self.lref.get();
        unsafe { ffi::luaL_unref(self.state.as_ptr(), self.t, lref) }
    }
}
