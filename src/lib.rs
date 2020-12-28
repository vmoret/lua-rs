#![feature(box_syntax)]
#![feature(const_raw_ptr_deref)]

#[macro_use]
extern crate log;

pub use self::state::State;

mod alloc;
mod ffi;
mod state;
