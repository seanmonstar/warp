use http;

use ::never::Never;
use ::reply::Reply;

/// Rejects a request with a default `400 Bad Request`.
#[inline]
pub fn reject() -> Rejection {
    bad_request()
}

#[inline]
pub fn bad_request() -> Rejection {
    Reason::BadRequest.into()
}

#[inline]
pub fn not_found() -> Rejection {
    Reason::NotFound.into()
}

#[inline]
pub fn server_error() -> Rejection {
    Reason::ServerError.into()
}

/// Rejection of a request by a [`Filter`](::Filter).
#[derive(Debug)]
pub struct Rejection {
    reason: Reason,
}

#[derive(Debug)]
pub(crate) enum Reason {
    NotFound,
    BadRequest,
    ServerError,
}

#[doc(hidden)]
impl From<Reason> for Rejection {
    #[inline]
    fn from(reason: Reason) -> Rejection {
        Rejection {
            reason,
        }
    }
}

impl From<Never> for Rejection {
    #[inline]
    fn from(never: Never) -> Rejection {
        match never {}
    }
}

impl Reply for Rejection {
    fn into_response(self) -> ::reply::Response {
        let code = match self.reason {
            Reason::NotFound => http::StatusCode::NOT_FOUND,
            Reason::BadRequest => http::StatusCode::BAD_REQUEST,
            Reason::ServerError => http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let mut res = http::Response::default();
        *res.status_mut() = code;
        res
    }
}

pub trait CombineRejection<E>: Send + Sized {
    type Rejection: ::std::fmt::Debug + From<Self> + From<E> + Send;
}

impl CombineRejection<Rejection> for Rejection {
    type Rejection = Rejection;
}

impl CombineRejection<Never> for Rejection {
    type Rejection = Rejection;
}

impl CombineRejection<Rejection> for Never {
    type Rejection = Rejection;
}

impl CombineRejection<Never> for Never {
    type Rejection = Never;
}

