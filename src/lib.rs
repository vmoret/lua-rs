#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

// pub use self::guard::StackGuard;
pub use self::error::{Error, Result};
pub use self::state::{State, types};
pub use self::stack::{Globals, GlobalsMut, Stack, Mode};

mod alloc;
pub mod de;
mod error;
mod ffi;
// mod guard;
mod lref;
pub mod ser;
mod stack;
mod state;