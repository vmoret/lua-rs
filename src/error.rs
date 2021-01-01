use std::fmt;

/// A specialized [`Result`](std::result::Result) type for Lua operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for Lua operations.
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
}

/// A list specifying general categories of Lua error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An invalid input was provided.
    InvalidInput,
    /// An invalid data was encountered.
    InvalidData,
    /// An error not in this list was encountered.
    Other,
}

impl ErrorKind {
    fn as_str(&self) -> &str {
        match *self {
            ErrorKind::InvalidInput => "invalid input",
            ErrorKind::InvalidData => "invalid data",
            ErrorKind::Other => "other error",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Error {
    /// Creates a new Lua error from a known kind of error as well as an arbitrary error payload.
    pub fn new<E>(kind: ErrorKind, error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ErrorKind, error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self { kind, error }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("kind", &self.kind)
            .field("error", &self.error)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

macro_rules! impl_from_errors {
    (($($ty:ty), *), $var:ident) => {$(
        impl From<$ty> for Error {
            fn from(error: $ty) -> Self {
                Self::new(ErrorKind::$var, error)
            }
        }
    )*};
}

impl_from_errors!(
    (
        std::ffi::NulError,
        std::io::Error,
        std::str::Utf8Error,
        std::string::FromUtf8Error
    ),
    InvalidData
);
