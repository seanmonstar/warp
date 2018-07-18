use std::error::Error as StdError;
use std::fmt;

use tungstenite::Error as WsError;

use never::Never;

/// Errors that can happen inside warp.
#[derive(Debug)]
pub struct Error(Kind);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Kind::Ws(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self.0 {
            Kind::Ws(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match self.0 {
            Kind::Ws(ref e) => e.cause(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Kind {
    Ws(WsError),
}

#[doc(hidden)]
impl From<Kind> for Error {
    fn from(kind: Kind) -> Error {
        Error(kind)
    }
}

impl From<Never> for Error {
    fn from(never: Never) -> Error {
        match never {}
    }
}

