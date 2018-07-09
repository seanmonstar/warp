//! Rejections

use http;

use ::never::Never;

pub(crate) use self::sealed::{CombineRejection, Reject};

/// Rejects a request with a default `400 Bad Request`.
#[inline]
pub fn reject() -> Rejection {
    bad_request()
}

/// Rejects a request with `400 Bad Request`.
#[inline]
pub fn bad_request() -> Rejection {
    Reason::BadRequest.into()
}

#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    Reason::MethodNotAllowed.into()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Reason::NotFound.into()
}

/// Rejects a request with `500 Internal Server Error`.
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
    BadRequest,
    NotFound,
    MethodNotAllowed,
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

impl Reject for Never {
    fn status(&self) -> http::StatusCode {
        match *self {}
    }

    fn into_response(self) -> ::reply::Response {
        match self {}
    }
}

impl Reject for Rejection {
    fn status(&self) -> http::StatusCode {
        match self.reason {
            Reason::BadRequest => http::StatusCode::BAD_REQUEST,
            Reason::NotFound => http::StatusCode::NOT_FOUND,
            Reason::MethodNotAllowed => http::StatusCode::METHOD_NOT_ALLOWED,
            Reason::ServerError => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn into_response(self) -> ::reply::Response {
        let code = self.status();

        let mut res = http::Response::default();
        *res.status_mut() = code;
        res
    }
}



mod sealed {
    use ::never::Never;
    use super::Rejection;

    pub trait Reject {
        fn status(&self) -> ::http::StatusCode;
        fn into_response(self) -> ::reply::Response;
    }

    fn _assert_object_safe() {
        fn _assert(_: &Reject) {}
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
}
