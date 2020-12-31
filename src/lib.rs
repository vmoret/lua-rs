#![feature(box_syntax)]

#[macro_use]
extern crate log;

pub use self::error::{Error, Result};
pub use self::state::{types, State};

mod alloc;
mod error;
mod ffi;
mod state;
