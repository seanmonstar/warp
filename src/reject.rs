//! Rejections
//!
//! Part of the power of the [`Filter`](Filter) system is being able to
//! reject a request from a filter chain. This allows for filters to be
//! combined with `or`, so that if one side of the chain finds that a request
//! doesn't fulfill its requirements, the other side can try to process
//! the request.
//!
//! Many of the built-in [`filters`](filters) will automatically reject
//! the request with a appropriate rejection. However, you can also build
//! new custom [`Filter`](Filter)s and still want other routes to be
//! matchable in the case a predicate doesn't hold.
//!
//! # Example
//!
//! ```
//! use warp::Filter;
//!
//! // Filter on `/:id`, but reject with 404 if the `id` is `0`.
//! let route = warp::path::param()
//!     .and_then(|id: u32| {
//!         if id == 0 {
//!             Err(warp::reject())
//!         } else {
//!             Ok("something since id is valid")
//!         }
//!     });
//! ```

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
    Reason::BAD_REQUEST.into()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Reason::empty().into()
}

// 405 Method Not Allowed
#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    Reason::METHOD_NOT_ALLOWED.into()
}

// 415 Unsupported Media Type
//
// Used by the body filters if the request payload content-type doesn't match
// what can be deserialized.
#[inline]
pub(crate) fn unsupported_media_type() -> Rejection {
    Reason::UNSUPPORTED_MEDIA_TYPE.into()
}

/// Rejects a request with `500 Internal Server Error`.
#[inline]
pub fn server_error() -> Rejection {
    Reason::SERVER_ERROR.into()
}

/// Rejection of a request by a [`Filter`](::Filter).
#[derive(Debug)]
pub struct Rejection {
    reason: Reason,
    cancel: Reason,
}

bitflags! {
    struct Reason: u8 {
        // NOT_FOUND = 0
        const BAD_REQUEST = 0b001;
        const METHOD_NOT_ALLOWED = 0b010;
        const UNSUPPORTED_MEDIA_TYPE = 0b100;
        const SERVER_ERROR = 0b1000;
    }
}

#[doc(hidden)]
impl From<Reason> for Rejection {
    #[inline]
    fn from(reason: Reason) -> Rejection {
        Rejection {
            reason,
            cancel: Reason::empty(),
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
        if self.reason.contains(Reason::SERVER_ERROR) {
            http::StatusCode::INTERNAL_SERVER_ERROR
        } else if self.reason.contains(Reason::METHOD_NOT_ALLOWED) {
            http::StatusCode::METHOD_NOT_ALLOWED
        } else if self.reason.contains(Reason::UNSUPPORTED_MEDIA_TYPE) {
            http::StatusCode::UNSUPPORTED_MEDIA_TYPE
        } else if self.reason.contains(Reason::BAD_REQUEST) {
            http::StatusCode::BAD_REQUEST
        } else {
            debug_assert!(self.reason.is_empty());
            http::StatusCode::NOT_FOUND
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

    pub trait Reject: ::std::fmt::Debug + Send {
        fn status(&self) -> ::http::StatusCode;
        fn into_response(self) -> ::reply::Response;
    }

    fn _assert_object_safe() {
        fn _assert(_: &Reject) {}
    }

    pub trait CombineRejection<E>: Send + Sized {
        type Rejection: Reject + From<Self> + From<E>;

        fn combine(self, other: E) -> Self::Rejection;
        fn cancel(self, other: E) -> Self::Rejection;
    }

    impl CombineRejection<Rejection> for Rejection {
        type Rejection = Rejection;

        fn combine(self, other: Rejection) -> Self::Rejection {
            let reason = (self.reason - other.cancel)
                | (other.reason - self.cancel);
            let cancel = self.cancel | other.cancel;

            Rejection {
                reason,
                cancel,
            }
            /*
            Rejection {
                reason: self.reason | other.reason,
            }
            */
        }

        fn cancel(mut self, other: Rejection) -> Self::Rejection {
            self.cancel.insert(other.reason);
            self
        }
    }

    impl CombineRejection<Never> for Rejection {
        type Rejection = Rejection;

        fn combine(self, other: Never) -> Self::Rejection {
            match other {}
        }

        fn cancel(self, other: Never) -> Self::Rejection {
            match other {}
        }
    }

    impl CombineRejection<Rejection> for Never {
        type Rejection = Rejection;

        fn combine(self, _: Rejection) -> Self::Rejection {
            match self {}
        }

        fn cancel(self, _: Rejection) -> Self::Rejection {
            match self {}
        }
    }

    impl CombineRejection<Never> for Never {
        type Rejection = Never;

        fn combine(self, _: Never) -> Self::Rejection {
            match self {}
        }

        fn cancel(self, _: Never) -> Self::Rejection {
            match self {}
        }
    }
}
