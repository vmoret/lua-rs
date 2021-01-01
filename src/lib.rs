#![feature(box_syntax)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]

#[macro_use]
extern crate log;

pub use self::error::{Error, ErrorKind, Result};
pub use self::state::{types, State};

#[doc(hidden)]
pub mod ffi;

mod alloc;
mod error;
pub mod state;
