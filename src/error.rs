use std::error::Error as StdError;
use std::convert::Infallible;
use std::fmt;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Errors that can happen inside warp.
pub struct Error(BoxError);

impl Error {
    pub(crate) fn new<E: Into<BoxError>>(err: E) -> Error {
        Error(err.into())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Skip showing worthless `Error { .. }` wrapper.
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl StdError for Error {}

impl From<Infallible> for Error {
    fn from(infallible: Infallible) -> Error {
        match infallible {}
    }
}

#[test]
fn error_size_of() {
    assert_eq!(
        ::std::mem::size_of::<Error>(),
        ::std::mem::size_of::<usize>() * 2
    );
}
