use std::error::Error as StdError;
use std::fmt;

use hyper::Error as HyperError;
use tungstenite::Error as WsError;

use never::Never;

/// Errors that can happen inside warp.
pub struct Error(Box<Kind>);

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Skip showing worthless `Error { .. }` wrapper.
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.as_ref() {
            Kind::Hyper(ref e) => fmt::Display::fmt(e, f),
            Kind::Ws(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self.0.as_ref() {
            Kind::Hyper(ref e) => e.description(),
            Kind::Ws(ref e) => e.description(),
        }
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn StdError> {
        match self.0.as_ref() {
            Kind::Hyper(ref e) => e.cause(),
            Kind::Ws(ref e) => e.cause(),
        }
    }
}

pub(crate) enum Kind {
    Hyper(HyperError),
    Ws(WsError),
}

impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Kind::Hyper(ref e) => fmt::Debug::fmt(e, f),
            Kind::Ws(ref e) => fmt::Debug::fmt(e, f),
        }
    }
}

#[doc(hidden)]
impl From<Kind> for Error {
    fn from(kind: Kind) -> Error {
        Error(Box::new(kind))
    }
}

impl From<Never> for Error {
    fn from(never: Never) -> Error {
        match never {}
    }
}

#[test]
fn error_size_of() {
    assert_eq!(
        ::std::mem::size_of::<Error>(),
        ::std::mem::size_of::<usize>()
    );
}
