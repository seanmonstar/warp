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
    Reason::BAD_REQUEST.into()
}

#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    Reason::METHOD_NOT_ALLOWED.into()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Reason::empty().into()
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
        const SERVER_ERROR = 0b100;
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

    pub trait Reject {
        fn status(&self) -> ::http::StatusCode;
        fn into_response(self) -> ::reply::Response;
    }

    fn _assert_object_safe() {
        fn _assert(_: &Reject) {}
    }

    pub trait CombineRejection<E>: Send + Sized {
        type Rejection: ::std::fmt::Debug + From<Self> + From<E> + Send;

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
