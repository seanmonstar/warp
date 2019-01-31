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
//! // Filter on `/:id`, but reject with 404 if the `id` is `0`.
//! let route = warp::path::param()
//!     .and_then(|id: u32| {
//!         if id == 0 {
//!             Err(warp::reject::not_found())
//!         } else {
//!             Ok("something since id is valid")
//!         }
//!     });
//! ```

use std::any::Any;
use std::error::Error as StdError;
use std::fmt;

use http::{
    self,
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use hyper::Body;

use never::Never;

pub(crate) use self::sealed::{CombineRejection, Reject};

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn reject() -> Rejection {
    not_found()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Rejection {
        reason: Reason::NotFound,
    }
}

// 405 Method Not Allowed
#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    known(MethodNotAllowed)
}

// 411 Length Required
#[inline]
pub(crate) fn length_required() -> Rejection {
    known(LengthRequired)
}

// 413 Payload Too Large
#[inline]
pub(crate) fn payload_too_large() -> Rejection {
    known(PayloadTooLarge)
}

// 415 Unsupported Media Type
//
// Used by the body filters if the request payload content-type doesn't match
// what can be deserialized.
#[inline]
pub(crate) fn unsupported_media_type() -> Rejection {
    known(UnsupportedMediaType)
}

/// Rejects a request with a custom cause.
///
/// A [`recover`][] filter should convert this `Rejection` into a `Reply`,
/// or else this will be returned as a `500 Internal Server Error`.
///
/// [`recover`]: ../../trait.Filter.html#method.recover
pub fn custom<T: Any + fmt::Debug + Send + Sync>(err: T) -> Rejection {
    Rejection::custom(Box::new(err))
}

trait Cause: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn fmt_debug(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T> Cause for T
where
    T: Any + fmt::Debug + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn fmt_debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl dyn Cause {
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

impl fmt::Debug for Box<Cause> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (&**self).fmt_debug(f)
    }
}

pub(crate) fn known<T: Into<Known>>(err: T) -> Rejection {
    Rejection::known(err.into())
}

/// Rejection of a request by a [`Filter`](::Filter).
///
/// See the [`reject`](reject) documentation for more.
pub struct Rejection {
    reason: Reason,
}

enum Reason {
    NotFound,
    Other(Box<Rejections>),
}

enum Rejections {
    //TODO(v0.2): For 0.1, this needs to hold a Box<StdError>, in order to support
    //cause() returning a `&Box<StdError>`. With 0.2, this should no longer need
    //to be boxed.
    Known(Known),
    Custom(Box<Cause>),
    Combined(Box<Rejections>, Box<Rejections>),
}

macro_rules! enum_known {
    ($($var:ident($ty:path),)+) => (
        pub(crate) enum Known {
            $(
            $var($ty),
            )+
        }

        impl Known {
            fn inner_as_any(&self) -> &Any {
                match *self {
                    $(
                    Known::$var(ref t) => t,
                    )+
                }
            }
        }

        impl fmt::Debug for Known {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                    Known::$var(ref t) => t.fmt(f),
                    )+
                }
            }
        }

        impl fmt::Display for Known {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                    Known::$var(ref t) => t.fmt(f),
                    )+
                }
            }
        }

        $(
        #[doc(hidden)]
        impl From<$ty> for Known {
            fn from(ty: $ty) -> Known {
                Known::$var(ty)
            }
        }
        )+
    );
}


enum_known! {
    MethodNotAllowed(MethodNotAllowed),
    InvalidHeader(::header::InvalidHeader),
    MissingHeader(::header::MissingHeader),
    MissingCookie(::cookie::MissingCookie),
    InvalidQuery(::query::InvalidQuery),
    LengthRequired(LengthRequired),
    PayloadTooLarge(PayloadTooLarge),
    UnsupportedMediaType(UnsupportedMediaType),
    BodyReadError(::body::BodyReadError),
    BodyDeserializeError(::body::BodyDeserializeError),
    CorsForbidden(::cors::CorsForbidden),
    MissingConnectionUpgrade(::ws::MissingConnectionUpgrade),
    MissingExtension(::ext::MissingExtension),
    ReplyHttpError(::reply::ReplyHttpError),
    ReplyJsonError(::reply::ReplyJsonError),
    BodyConsumedMultipleTimes(::body::BodyConsumedMultipleTimes),
    FsNeedsTokioThreadpool(::fs::FsNeedsTokioThreadpool),
}


impl Rejection {
    fn known(known: Known) -> Self {
        Rejection {
            reason: Reason::Other(Box::new(Rejections::Known(known))),
        }
    }

    fn custom(other: Box<Cause>) -> Self {
        Rejection {
            reason: Reason::Other(Box::new(Rejections::Custom(other))),
        }
    }

    /// Searches this `Rejection` for a specific cause.
    ///
    /// A `Rejection` will accumulate causes over a `Filter` chain. This method
    /// can search through them and return the first cause of this type.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io;
    ///
    /// let err = io::Error::new(
    ///     io::ErrorKind::Other,
    ///     "could be any std::error::Error"
    /// );
    /// let reject = warp::reject::custom(err);
    ///
    /// if let Some(cause) = reject.find_cause::<io::Error>() {
    ///    println!("found the io::Error: {}", cause);
    /// }
    /// ```
    pub fn find_cause<T: 'static>(&self) -> Option<&T> {
        if let Reason::Other(ref rejections) = self.reason {
            return rejections.find_cause();
        }
        None
    }

    /// Returns true if this Rejection was made via `warp::reject::not_found`.
    ///
    /// # Example
    ///
    /// ```
    /// let rejection = warp::reject::not_found();
    ///
    /// assert!(rejection.is_not_found());
    /// ```
    pub fn is_not_found(&self) -> bool {
        if let Reason::NotFound = self.reason {
            true
        } else {
            false
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
    fn status(&self) -> StatusCode {
        match *self {}
    }

    fn into_response(&self) -> ::reply::Response {
        match *self {}
    }
}

impl Reject for Rejection {
    fn status(&self) -> StatusCode {
        match self.reason {
            Reason::NotFound => StatusCode::NOT_FOUND,
            Reason::Other(ref other) => other.status(),
        }
    }

    fn into_response(&self) -> ::reply::Response {
        match self.reason {
            Reason::NotFound => {
                let mut res = http::Response::default();
                *res.status_mut() = StatusCode::NOT_FOUND;
                res
            }
            Reason::Other(ref other) => other.into_response(),
        }
    }
}

impl fmt::Debug for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Rejection").field(&self.reason).finish()
    }
}

impl fmt::Debug for Reason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Reason::NotFound => f.write_str("NotFound"),
            Reason::Other(ref other) => fmt::Debug::fmt(other, f),
        }
    }
}

// ===== Rejections =====

impl Rejections {
    fn status(&self) -> StatusCode {
        match *self {
            Rejections::Known(ref k) => match *k {
                Known::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
                Known::InvalidHeader(_) |
                Known::MissingHeader(_) |
                Known::MissingCookie(_) |
                Known::InvalidQuery(_) |
                Known::BodyReadError(_) |
                Known::BodyDeserializeError(_) |
                Known::MissingConnectionUpgrade(_) => StatusCode::BAD_REQUEST,
                Known::LengthRequired(_) => StatusCode::LENGTH_REQUIRED,
                Known::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
                Known::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Known::CorsForbidden(_) => StatusCode::FORBIDDEN,
                Known::MissingExtension(_) |
                Known::ReplyHttpError(_) |
                Known::ReplyJsonError(_) |
                Known::BodyConsumedMultipleTimes(_) |
                Known::FsNeedsTokioThreadpool(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Rejections::Custom(..) => StatusCode::INTERNAL_SERVER_ERROR,
            Rejections::Combined(ref a, ref b) => preferred(a, b).status(),
        }
    }

    fn into_response(&self) -> ::reply::Response {
        match *self {
            Rejections::Known(ref e) => {
                let mut res = http::Response::new(Body::from(e.to_string()));
                *res.status_mut() = self.status();
                res.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                res
            }
            Rejections::Custom(ref e) => {
                error!(
                    "unhandled custom rejection, returning 500 response: {:?}",
                    e
                );
                let body = format!("Unhandled rejection: {:?}", e);
                let mut res = http::Response::new(Body::from(body));
                *res.status_mut() = self.status();
                res.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                res
            }
            Rejections::Combined(ref a, ref b) => preferred(a, b).into_response(),
        }
    }

    pub fn find_cause<T: 'static>(&self) -> Option<&T> {
        match *self {
            Rejections::Known(ref e) => e.inner_as_any().downcast_ref(),
            Rejections::Custom(ref e) => e.downcast_ref(),
            Rejections::Combined(ref a, ref b) => a.find_cause().or_else(|| b.find_cause()),
        }
    }
}

fn preferred<'a>(a: &'a Rejections, b: &'a Rejections) -> &'a Rejections {
    // Compare status codes, with this priority:
    // - NOT_FOUND is lowest
    // - METHOD_NOT_ALLOWED is second
    // - if one status code is greater than the other
    // - otherwise, prefer A...
    match (a.status(), b.status()) {
        (_, StatusCode::NOT_FOUND) => a,
        (StatusCode::NOT_FOUND, _) => b,
        (_, StatusCode::METHOD_NOT_ALLOWED) => a,
        (StatusCode::METHOD_NOT_ALLOWED, _) => b,
        (sa, sb) if sa < sb => b,
        _ => a,
    }
}

impl fmt::Debug for Rejections {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Rejections::Known(ref e) => fmt::Debug::fmt(e, f),
            Rejections::Custom(ref e) => f.debug_tuple("Custom").field(e).finish(),
            Rejections::Combined(ref a, ref b) => {
                f.debug_tuple("Combined").field(a).field(b).finish()
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct MethodNotAllowed;

impl fmt::Display for MethodNotAllowed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("HTTP method not allowed")
    }
}

impl StdError for MethodNotAllowed {
    fn description(&self) -> &str {
        "HTTP method not allowed"
    }
}

#[derive(Debug)]
pub(crate) struct LengthRequired;

impl fmt::Display for LengthRequired {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("A content-length header is required")
    }
}

impl StdError for LengthRequired {
    fn description(&self) -> &str {
        "A content-length header is required"
    }
}

#[derive(Debug)]
pub(crate) struct PayloadTooLarge;

impl fmt::Display for PayloadTooLarge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("The request payload is too large")
    }
}

impl StdError for PayloadTooLarge {
    fn description(&self) -> &str {
        "The request payload is too large"
    }
}

#[derive(Debug)]
pub(crate) struct UnsupportedMediaType;

impl fmt::Display for UnsupportedMediaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("The request's content-type is not supported")
    }
}

impl StdError for UnsupportedMediaType {
    fn description(&self) -> &str {
        "The request's content-type is not supported"
    }
}

mod sealed {
    use super::{Reason, Rejection, Rejections};
    use http::StatusCode;
    use never::Never;
    use std::fmt;

    pub trait Reject: fmt::Debug + Send + Sync {
        fn status(&self) -> StatusCode;
        fn into_response(&self) -> ::reply::Response;
    }

    fn _assert_object_safe() {
        fn _assert(_: &Reject) {}
    }

    pub trait CombineRejection<E>: Send + Sized {
        type Rejection: Reject + From<Self> + From<E> + Into<Rejection>;

        fn combine(self, other: E) -> Self::Rejection;
    }

    impl CombineRejection<Rejection> for Rejection {
        type Rejection = Rejection;

        fn combine(self, other: Rejection) -> Self::Rejection {
            let reason = match (self.reason, other.reason) {
                (Reason::Other(left), Reason::Other(right)) => {
                    Reason::Other(Box::new(Rejections::Combined(left, right)))
                }
                (Reason::Other(other), Reason::NotFound)
                | (Reason::NotFound, Reason::Other(other)) => {
                    // ignore the NotFound
                    Reason::Other(other)
                }
                (Reason::NotFound, Reason::NotFound) => Reason::NotFound,
            };

            Rejection { reason }
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
    use super::*;
    use http::StatusCode;

    #[derive(Debug, PartialEq)]
    struct Left;

    #[derive(Debug, PartialEq)]
    struct Right;

    #[test]
    fn rejection_status() {
        assert_eq!(not_found().status(), StatusCode::NOT_FOUND);
        assert_eq!(
            method_not_allowed().status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(length_required().status(), StatusCode::LENGTH_REQUIRED);
        assert_eq!(payload_too_large().status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            unsupported_media_type().status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(custom("boom").status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn combine_rejection_causes_with_some_left_and_none_right() {
        let left = custom(Left);
        let right = not_found();
        let reject = left.combine(right);
        let resp = reject.into_response();

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: Left")
    }

    #[test]
    fn combine_rejection_causes_with_none_left_and_some_right() {
        let left = not_found();
        let right = custom(Right);
        let reject = left.combine(right);
        let resp = reject.into_response();

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: Right")
    }

    #[test]
    fn unhandled_customs() {
        let reject = not_found().combine(custom(Right));

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: Right");

        // There's no real way to determine which is worse, since both are a 500,
        // so pick the first one.
        let reject = custom(Left).combine(custom(Right));

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: Left");

        // With many rejections, custom still is top priority.
        let reject = not_found()
            .combine(not_found())
            .combine(not_found())
            .combine(custom(Right))
            .combine(not_found());

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: Right");
    }

    fn response_body_string(resp: ::reply::Response) -> String {
        use futures::{Async, Future, Stream};

        let (_, body) = resp.into_parts();
        match body.concat2().poll() {
            Ok(Async::Ready(chunk)) => String::from_utf8_lossy(&chunk).to_string(),
            err => unreachable!("{:?}", err),
        }
    }

    #[test]
    fn find_cause() {
        let rej = custom(Left);

        assert_eq!(rej.find_cause::<Left>(), Some(&Left));

        let rej = rej.combine(method_not_allowed());

        assert_eq!(rej.find_cause::<Left>(), Some(&Left));
        assert!(
            rej.find_cause::<MethodNotAllowed>().is_some(),
            "MethodNotAllowed"
        );
    }

    /*
    XXX: Should this test be true?
    #[test]
    fn find_cause_box_error() {
        use std::io;

        let err: Box<StdError + Send + Sync> = Box::new(io::Error::new(io::ErrorKind::Other, "boom"));
        let rej = custom(err);

        assert!(rej.find_cause::<io::Error>().is_some());
    }
    */

    #[test]
    fn size_of_rejection() {
        assert_eq!(
            ::std::mem::size_of::<Rejection>(),
            ::std::mem::size_of::<usize>(),
        );
    }
}
