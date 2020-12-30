use std::fmt;

/// A specialized [`Result`](std::result::Result) type for Lua operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for Lua operations of the [`Push`], Pull, Pop, and associated
/// traits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    /// A custom error.
    Custom(String),
    /// An invalid input was provided for `name`.
    InvalidInput { 
        /// The name of the argument.
        name: String, 
        /// The string describing the error.
        error: String,
    },
    /// An invalid Lua integer was encountered.
    InvalidInteger,
    /// An invalid Lua number was encountered.
    InvalidNumber,
    /// An invalid Lua string was encountered.
    InvalidString,
    /// An invalid Lua type was encountered.
    InvalidType(i32),
    /// The Lua stack has overflown.
    StackOverflow,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("error")
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Custom(msg.to_string())
    }
}
