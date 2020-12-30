use std::{borrow::Borrow, ops::Deref};

use super::state::State;

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
}

impl Deref for Stack {
    type Target = State;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl ToOwned for Stack {
    type Owned = StackMut;
    fn to_owned(&self) -> Self::Owned {
        StackMut { state: self.state.clone() }
    }
}
