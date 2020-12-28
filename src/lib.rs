#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

pub use self::guard::StackGuard;
pub use self::state::State;

mod alloc;
mod ffi;
mod guard;
pub mod ser;
mod state;
