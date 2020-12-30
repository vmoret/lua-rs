#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

pub use self::guard::StackGuard;
pub use self::state::{Globals, GlobalsMut, Mode, State, types};

mod alloc;
pub mod de;
mod ffi;
mod guard;
mod lref;
pub mod ser;
mod state;
