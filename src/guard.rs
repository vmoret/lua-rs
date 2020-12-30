use std::ops::{Add, Deref};

use crate::state::State;

/// A StackGuard guards the stack size of a Lua [`State`].
/// 
/// When the guard goes out of scope it will check the size and drop any elements above the 
/// high-water mark set at initialization.
pub struct StackGuard {
    state: State,
    top: i32,
}

impl StackGuard {
    /// Creates a new `StackGuard` using the current stack size as high-water mark.
    pub fn new(state: State) -> Self {
        let top = state.as_stack().top();
        Self { state, top }
    }
}

impl Drop for StackGuard {
    fn drop(&mut self) {
        let top = self.as_stack().top();
        if top > self.top {
            // remove the items above the high-water mark.
            unsafe { crate::ffi::lua_settop(self.state.as_ptr(), self.top) };
        } else if top < self.top {
            // abort here as it's considered a programming error to have less
            // elements on the stack while using this guard.
            std::process::abort();
        }
    }
}

impl Add<i32> for StackGuard {
    type Output = Self;
    fn add(self, rhs: i32) -> Self::Output {
        Self {
            top: self.top + rhs,
            state: self.state.clone(),
        }
    }
}

impl Deref for StackGuard {
    type Target = State;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}