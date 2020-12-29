#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

pub use self::guard::StackGuard;
pub use self::state::{State, Mode};

mod alloc;
pub mod de;
mod ffi;
mod guard;
mod lref;
pub mod ser;
mod state;
