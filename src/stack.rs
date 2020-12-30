use std::{borrow::Borrow, ops::Deref};

use super::{ffi, state::State};

/// A mutable access to the Lua stack.
#[derive(Debug, Clone)]
pub struct StackMut {
    state: State,
}

impl Deref for StackMut {
    type Target = Stack;
    fn deref(&self) -> &Self::Target {
        Stack::new(&self.state)
    }
}

impl From<State> for StackMut {
    /// Converts a `State` into a `StackMut`
    ///
    /// This conversion does not allocate or copy memory.
    #[inline]
    fn from(state: State) -> Self {
        Self { state }
    }
}

impl Borrow<Stack> for StackMut {
    fn borrow(&self) -> &Stack {
        self.deref()
    }
}

impl From<StackMut> for State {
    fn from(this: StackMut) -> State {
        this.state
    }
}

/// An immutable access to the Lua stack.
#[derive(Debug)]
pub struct Stack {
    state: State,
}

impl Stack {
    /// Directly wraps a [`State`] as a `Stack` reference.
    /// 
    /// This is a cost-free conversion.
    pub fn new<S: AsRef<State>>(state: &S) -> &Stack {
        unsafe { &*(state.as_ref() as *const State as *const Stack) }
    }

    /// Gets a mutable pointer to the Lua state pointer.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut ffi::lua_State {
        self.state.as_ptr()
    }

    /// Ensures that the stack has space for at least `n` extra elements, that is, that you can
    /// safely push up to `n` values into it. It returns `false` if it cannot fulfill the request,
    /// either because it would cause the stack to be greater than a fixed maximum size (typically
    /// at least several thousand elements) or because it cannot allocate memory for the extra space.
    /// 
    /// This function never shrinks the stack; if the stack already has space for the extra elements,
    /// it is left unchanged.
    pub fn reserve(&self, n: i32) -> bool {
        unsafe { ffi::lua_checkstack(self.as_ptr(), n) != 0 }
    }

    /// Returns the index of the top element in the stack.
    /// 
    /// Because indices start at 1, this result is equal to the number of elements in the stack; in 
    /// particular, 0 means an empty stack.
    pub fn top(&self) -> i32 {
        unsafe { ffi::lua_gettop(self.as_ptr()) }
    }
}

impl ToOwned for Stack {
    type Owned = StackMut;
    fn to_owned(&self) -> Self::Owned {
        StackMut { state: self.state.clone() }
    }
}
