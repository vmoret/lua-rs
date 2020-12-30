#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

// pub use self::guard::StackGuard;
pub use self::error::{Error, Result};
pub use self::stack::{Globals, GlobalsMut, Mode, Pull, Push, Stack};
pub use self::state::{types, State};

mod alloc;
mod de;
mod error;
mod ffi;
// mod guard;
mod lref;
mod ser;
mod stack;
mod state;
