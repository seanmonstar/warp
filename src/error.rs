use futures::future;
use http;

use never::Never;
use reply::Reply;

/// dox?
#[derive(Debug)]
pub struct Error(Kind);

#[derive(Debug)]
pub(crate) enum Kind {
    NotFound,
    BadRequest,
    Ws,
}

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

impl Reply for Error {
    type Future = future::FutureResult<::reply::Response, Never>;
    fn into_response(self) -> Self::Future {
        let code = match self.0 {
            Kind::NotFound => http::StatusCode::NOT_FOUND,
            Kind::BadRequest => http::StatusCode::BAD_REQUEST,
            Kind::Ws => http::StatusCode::BAD_REQUEST,
        };

        let mut res = http::Response::default();
        *res.status_mut() = code;
        future::ok(res)
    }
}

pub trait CombineError<E>: Send + Sized {
    type Error: ::std::fmt::Debug + From<Self> + From<E> + Send;
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
