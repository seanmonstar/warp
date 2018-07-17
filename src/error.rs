use never::Never;

/// Errors that can happen inside warp.
#[derive(Debug)]
pub struct Error(Kind);

#[derive(Debug)]
pub(crate) enum Kind {
    Ws,
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

