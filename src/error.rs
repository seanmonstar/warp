use never::Never;

/// dox?
#[derive(Debug)]
pub struct Error(pub(crate) ());

impl From<Never> for Error {
    fn from(never: Never) -> Error {
        match never {}
    }
}

pub trait CombineError<E>: Send + Sized {
    type Error: From<Self> + From<E> + Send;
}

impl CombineError<Error> for Error {
    type Error = Error;
}

impl CombineError<Never> for Error {
    type Error = Error;
}

impl CombineError<Error> for Never {
    type Error = Error;
}

impl CombineError<Never> for Never {
    type Error = Never;
}
