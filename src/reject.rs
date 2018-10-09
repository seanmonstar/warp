//! Rejections
//!
//! Part of the power of the [`Filter`](../trait.Filter.html) system is being able to
//! reject a request from a filter chain. This allows for filters to be
//! combined with `or`, so that if one side of the chain finds that a request
//! doesn't fulfill its requirements, the other side can try to process
//! the request.
//!
//! Many of the built-in [`filters`](../filters) will automatically reject
//! the request with an appropriate rejection. However, you can also build
//! new custom [`Filter`](../trait.Filter.html)s and still want other routes to be
//! matchable in the case a predicate doesn't hold.
//!
//! # Example
//!
//! ```
//! use warp::Filter;
//!
//! // Filter on `/:id`, but reject with 400 if the `id` is `0`.
//! let route = warp::path::param()
//!     .and_then(|id: u32| {
//!         if id == 0 {
//!             Err(warp::reject())
//!         } else {
//!             Ok("something since id is valid")
//!         }
//!     });
//! ```

use std::error::Error as StdError;

use http;
use serde;
use serde_json;

use ::never::Never;

pub(crate) use self::sealed::{CombineRejection, Reject};

/// Error cause for a rejection.
pub type Cause = Box<StdError + Send + Sync>;

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

/// Rejects a request with `403 Forbidden`
#[inline]
pub fn forbidden() -> Rejection {
    Reason::FORBIDDEN.into()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Reason::empty().into()
}

/// Rejects a request with `408 Request Timeout`
#[inline]
pub fn request_timeout() -> Rejection {
    Reason::REQUEST_TIMEOUT.into()
}

// 405 Method Not Allowed
#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    Reason::METHOD_NOT_ALLOWED.into()
}

// 411 Length Required
#[inline]
pub(crate) fn length_required() -> Rejection {
    Reason::LENGTH_REQUIRED.into()
}

// 413 Payload Too Large
#[inline]
pub(crate) fn payload_too_large() -> Rejection {
    Reason::PAYLOAD_TOO_LARGE.into()
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
    cause: Option<Cause>,
}

bitflags! {
    struct Reason: u8 {
        // NOT_FOUND = 0
        const BAD_REQUEST            = 0b00000001;
        const METHOD_NOT_ALLOWED     = 0b00000010;
        const LENGTH_REQUIRED        = 0b00000100;
        const PAYLOAD_TOO_LARGE      = 0b00001000;
        const UNSUPPORTED_MEDIA_TYPE = 0b00010000;
        const FORBIDDEN              = 0b00100000;
        const REQUEST_TIMEOUT        = 0b01000000;

        // SERVER_ERROR has to be the last reason, to avoid shadowing it when combining rejections
        const SERVER_ERROR           = 0b10000000;
    }
}

impl Rejection {
    /// Return the HTTP status code that this rejection represents.
    pub fn status(&self) -> http::StatusCode {
        Reject::status(self)
    }

    /// Add given `err` into `Rejection`.
    pub fn with<E>(self, err: E) -> Self
    where
        E: Into<Cause>,
    {
        let cause = Some(err.into());
        Self {
            cause,
            .. self
        }
    }

    /// Returns a json response for this rejection.
    pub fn json(&self) -> ::reply::Response {
        use http::header::{CONTENT_TYPE, HeaderValue};
        use hyper::Body;

        let code = self.status();
        let mut res = http::Response::default();
        *res.status_mut() = code;

        res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        *res.body_mut() = match serde_json::to_string(&self) {
            Ok(body) => Body::from(body),
            Err(_) => Body::from("{}"),
        };

        res
    }

    /// Returns an error cause.
    pub fn cause(&self) -> Option<&Cause> {
        if let Some(ref err) = self.cause {
            return Some(&err)
        }
        None
    }

    /// Turn into cause of type `T`.
    pub fn into_cause<T>(self) -> Result<Box<T>, Self>
    where
        T: StdError + Send + Sync + 'static
    {
        let err = match self.cause {
            Some(err) => err,
            None => return Err(self)
        };

        match err.downcast::<T>() {
            Ok(err) => Ok(err),
            Err(other) => Err(Rejection {
                reason: self.reason,
                cause: Some(other)
            })
        }
    }
}

#[doc(hidden)]
impl From<Reason> for Rejection {
    #[inline]
    fn from(reason: Reason) -> Rejection {
        Rejection {
            reason,
            cause: None,
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

    fn cause(&self) -> Option<&Cause> {
        None
    }
}

impl Reject for Rejection {
    fn status(&self) -> http::StatusCode {
        if self.reason.contains(Reason::SERVER_ERROR) {
            http::StatusCode::INTERNAL_SERVER_ERROR
        } else if self.reason.contains(Reason::FORBIDDEN) {
            http::StatusCode::FORBIDDEN
        } else if self.reason.contains(Reason::UNSUPPORTED_MEDIA_TYPE) {
            http::StatusCode::UNSUPPORTED_MEDIA_TYPE
        } else if self.reason.contains(Reason::LENGTH_REQUIRED) {
            http::StatusCode::LENGTH_REQUIRED
        } else if self.reason.contains(Reason::PAYLOAD_TOO_LARGE) {
            http::StatusCode::PAYLOAD_TOO_LARGE
        } else if self.reason.contains(Reason::BAD_REQUEST) {
            http::StatusCode::BAD_REQUEST
        } else if self.reason.contains(Reason::METHOD_NOT_ALLOWED) {
            http::StatusCode::METHOD_NOT_ALLOWED
        } else if self.reason.contains(Reason::REQUEST_TIMEOUT) {
            http::StatusCode::REQUEST_TIMEOUT
        } else {
            debug_assert!(self.reason.is_empty());
            http::StatusCode::NOT_FOUND
        }
    }

    fn into_response(self) -> ::reply::Response {
        use http::header::{CONTENT_TYPE, HeaderValue};
        use hyper::Body;

        let code = self.status();
        let mut res = http::Response::default();
        *res.status_mut() = code;

        match self.cause {
            Some(err) => {
                let bytes = format!("{}", err);
                res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                *res.body_mut() = Body::from(bytes);
            },
            None => {}
        }

        res
    }

    #[inline]
    fn cause(&self) -> Option<&Cause> {
        Rejection::cause(&self)
    }
}

impl serde::Serialize for Rejection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        let err = match self.cause {
            Some(ref err) => err,
            None => return map.end()
        };

        map.serialize_key("description").and_then(|_| map.serialize_value(err.description()))?;
        map.serialize_key("message").and_then(|_| map.serialize_value(&err.to_string()))?;
        map.end()
    }
}

mod sealed {
    use ::never::Never;
    use super::{Cause, Rejection};

    pub trait Reject: ::std::fmt::Debug + Send {
        fn status(&self) -> ::http::StatusCode;
        fn into_response(self) -> ::reply::Response;
        fn cause(&self) -> Option<&Cause>;
    }

    fn _assert_object_safe() {
        fn _assert(_: &Reject) {}
    }

    pub trait CombineRejection<E>: Send + Sized {
        type Rejection: Reject + From<Self> + From<E>;

        fn combine(self, other: E) -> Self::Rejection;
    }

    impl CombineRejection<Rejection> for Rejection {
        type Rejection = Rejection;

        fn combine(self, other: Rejection) -> Self::Rejection {
            let reason = self.reason | other.reason;
            let cause = if self.reason > other.reason {
                self.cause
            } else {
                other.cause
            };

            Rejection {
                reason,
                cause
            }
        }
    }

    impl CombineRejection<Never> for Rejection {
        type Rejection = Rejection;

        fn combine(self, other: Never) -> Self::Rejection {
            match other {}
        }
    }

    impl CombineRejection<Rejection> for Never {
        type Rejection = Rejection;

        fn combine(self, _: Rejection) -> Self::Rejection {
            match self {}
        }
    }

    impl CombineRejection<Never> for Never {
        type Rejection = Never;

        fn combine(self, _: Never) -> Self::Rejection {
            match self {}
        }
    }
}

#[cfg(test)]
mod tests {
    use http::header::{CONTENT_TYPE};

    use super::*;
    use http::StatusCode;

    #[test]
    fn rejection_status() {
        assert_eq!(bad_request().status(), StatusCode::BAD_REQUEST);
        assert_eq!(forbidden().status(), StatusCode::FORBIDDEN);
        assert_eq!(not_found().status(), StatusCode::NOT_FOUND);
        assert_eq!(method_not_allowed().status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(length_required().status(), StatusCode::LENGTH_REQUIRED);
        assert_eq!(payload_too_large().status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(unsupported_media_type().status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(server_error().status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn combine_rejections() {
        let left = bad_request().with("left");
        let right = server_error().with("right");
        let reject = left.combine(right);

        assert_eq!(Reason::BAD_REQUEST | Reason::SERVER_ERROR, reject.reason);
        match reject.cause {
            Some(err) => assert_eq!("right", err.description()),
            err => unreachable!("{:?}", err)
        }
    }

    #[test]
    fn combine_rejection_causes_with_some_left_and_none_right() {
        let left = bad_request().with("left");
        let right = server_error();

        match left.combine(right).cause {
            None => {},
            err => unreachable!("{:?}", err)
        }
    }

    #[test]
    fn combine_rejection_causes_with_none_left_and_some_right() {
        let left = bad_request();
        let right = server_error().with("right");

        match left.combine(right).cause {
            Some(err) => assert_eq!("right", err.description()),
            err => unreachable!("{:?}", err)
        }
    }

    #[test]
    fn into_response_with_none_cause() {
        let resp = bad_request().into_response();
        assert_eq!(400, resp.status());
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
        assert_eq!("", response_body_string(resp))
    }

    #[test]
    fn into_response_with_some_cause() {
        let resp = server_error().with("boom").into_response();
        assert_eq!(500, resp.status());
        assert_eq!("text/plain", resp.headers().get(CONTENT_TYPE).unwrap());
        assert_eq!("boom", response_body_string(resp))
    }

    #[test]
    fn into_json_with_none_cause() {
        let resp = bad_request().json();
        assert_eq!(400, resp.status());
        assert_eq!("application/json", resp.headers().get(CONTENT_TYPE).unwrap());
        assert_eq!("{}", response_body_string(resp))
    }

    #[test]
    fn into_json_with_some_cause() {
        let resp = bad_request().with("boom").json();
        assert_eq!(400, resp.status());
        assert_eq!("application/json", resp.headers().get(CONTENT_TYPE).unwrap());
        let expected = "{\"description\":\"boom\",\"message\":\"boom\"}";
        assert_eq!(expected, response_body_string(resp))
    }

    fn response_body_string(resp: ::reply::Response) -> String {
        use futures::{Future, Stream, Async};

        let (_, body) = resp.into_parts();
        match body.concat2().poll() {
            Ok(Async::Ready(chunk)) => {
                String::from_utf8_lossy(&chunk).to_string()
            },
            err => unreachable!("{:?}", err)
        }
    }
}
