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

use std::error::Error as StdError;
use std::fmt;

use http::{
    self,
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use hyper::Body;
use serde;
use serde_json;

use never::Never;

pub(crate) use self::sealed::{CombineRejection, Reject};

//TODO(v0.2): This should just be `type Cause = StdError + Send + Sync + 'static`,
//and not include the `Box`.
#[doc(hidden)]
pub type Cause = Box<dyn StdError + Send + Sync>;

#[doc(hidden)]
#[deprecated(
    note = "this will be changed to return a NotFound rejection, use warp::reject::custom for custom bad requests"
)]
#[allow(deprecated)]
#[inline]
pub fn reject() -> Rejection {
    bad_request()
}

#[doc(hidden)]
#[deprecated(note = "use warp::reject::custom and Filter::recover to send a 401 error")]
pub fn bad_request() -> Rejection {
    Rejection::known_status(StatusCode::BAD_REQUEST)
}

#[doc(hidden)]
#[deprecated(note = "use warp::reject::custom and Filter::recover to send a 403 error")]
pub fn forbidden() -> Rejection {
    Rejection::known_status(StatusCode::FORBIDDEN)
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Rejection {
        reason: Reason::NotFound,
    }
}

// 400 Bad Request
#[inline]
pub(crate) fn invalid_query() -> Rejection {
    known(InvalidQuery(()))
}

// 400 Bad Request
#[inline]
pub(crate) fn missing_header(name: &'static str) -> Rejection {
    known(MissingHeader(name))
}

// 400 Bad Request
#[inline]
pub(crate) fn invalid_header(name: &'static str) -> Rejection {
    known(InvalidHeader(name))
}

// 400 Bad Request
#[inline]
pub(crate) fn missing_cookie(name: &'static str) -> Rejection {
    known(MissingCookie(name))
}

// 405 Method Not Allowed
#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    known(MethodNotAllowed(()))
}

// 411 Length Required
#[inline]
pub(crate) fn length_required() -> Rejection {
    known(LengthRequired(()))
}

// 413 Payload Too Large
#[inline]
pub(crate) fn payload_too_large() -> Rejection {
    known(PayloadTooLarge(()))
}

// 415 Unsupported Media Type
//
// Used by the body filters if the request payload content-type doesn't match
// what can be deserialized.
#[inline]
pub(crate) fn unsupported_media_type() -> Rejection {
    known(UnsupportedMediaType(()))
}

#[doc(hidden)]
#[deprecated(note = "use warp::reject::custom and Filter::recover to send a 500 error")]
pub fn server_error() -> Rejection {
    Rejection::known_status(StatusCode::INTERNAL_SERVER_ERROR)
}

/// Rejects a request with a custom cause.
///
/// A [`recover`][] filter should convert this `Rejection` into a `Reply`,
/// or else this will be returned as a `500 Internal Server Error`.
///
/// [`recover`]: ../../trait.Filter.html#method.recover
pub fn custom(err: impl Into<Cause>) -> Rejection {
    Rejection::custom(err.into())
}

pub(crate) fn known(err: impl Into<Cause>) -> Rejection {
    Rejection::known(err.into())
}

/// Rejection of a request by a [`Filter`](::Filter).
///
/// See the [`reject`](index.html) documentation for more.
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
    Known(Cause),
    KnownStatus(StatusCode),
    With(Rejection, Cause),
    Custom(Cause),
    Combined(Box<Rejections>, Box<Rejections>),
}

impl Rejection {
    fn known(other: Cause) -> Self {
        Rejection {
            reason: Reason::Other(Box::new(Rejections::Known(other))),
        }
    }

    fn known_status(status: StatusCode) -> Self {
        Rejection {
            reason: Reason::Other(Box::new(Rejections::KnownStatus(status))),
        }
    }

    fn custom(other: Cause) -> Self {
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
    pub fn find_cause<T: StdError + 'static>(&self) -> Option<&T> {
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

    #[doc(hidden)]
    pub fn status(&self) -> StatusCode {
        Reject::status(self)
    }

    #[doc(hidden)]
    #[deprecated(note = "Custom rejections should use `warp::reject::custom()`.")]
    pub fn with<E>(self, err: E) -> Self
    where
        E: Into<Cause>,
    {
        let cause = err.into();

        Self {
            reason: Reason::Other(Box::new(Rejections::With(self, cause))),
        }
    }

    #[doc(hidden)]
    #[deprecated(note = "Use warp::reply::json and warp::reply::with_status instead.")]
    pub fn json(&self) -> ::reply::Response {
        let code = self.status();
        let mut res = http::Response::default();
        *res.status_mut() = code;

        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        *res.body_mut() = match serde_json::to_string(&self) {
            Ok(body) => Body::from(body),
            Err(_) => Body::from("{}"),
        };

        res
    }

    /// Returns an optional error cause for this rejection.
    ///
    /// If this `Rejection` is actuall a combination of rejections, then the
    /// returned cause is determined by an internal ranking system. If you'd
    /// rather handle different causes with different priorities, use
    /// `find_cause`.
    ///
    /// # Note
    ///
    /// The return type will change from `&Box<Error>` to `&Error` in v0.2.
    /// This method isn't marked deprecated, however, since most people aren't
    /// actually using the `Box` part, and so a deprecation warning would just
    /// annoy people who didn't need to make any changes.
    pub fn cause(&self) -> Option<&Cause> {
        if let Reason::Other(ref err) = self.reason {
            return err.cause();
        }
        None
    }

    #[doc(hidden)]
    #[deprecated(note = "into_cause can no longer be provided")]
    pub fn into_cause<T>(self) -> Result<Box<T>, Self>
    where
        T: StdError + Send + Sync + 'static,
    {
        Err(self)
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

    fn cause(&self) -> Option<&Cause> {
        None
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

    fn cause(&self) -> Option<&Cause> {
        Rejection::cause(&self)
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

#[doc(hidden)]
#[deprecated(note = "Use warp::reply::json and warp::reply::with_status instead.")]
impl serde::Serialize for Rejection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        let err = match self.cause() {
            Some(err) => err,
            None => return map.end(),
        };

        map.serialize_key("description")
            .and_then(|_| map.serialize_value(err.description()))?;
        map.serialize_key("message")
            .and_then(|_| map.serialize_value(&err.to_string()))?;
        map.end()
    }
}

// ===== Rejections =====

impl Rejections {
    fn status(&self) -> StatusCode {
        match *self {
            Rejections::Known(ref e) => {
                if e.is::<MethodNotAllowed>() {
                    StatusCode::METHOD_NOT_ALLOWED
                } else if e.is::<InvalidHeader>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<MissingHeader>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<MissingCookie>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<InvalidQuery>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<LengthRequired>() {
                    StatusCode::LENGTH_REQUIRED
                } else if e.is::<PayloadTooLarge>() {
                    StatusCode::PAYLOAD_TOO_LARGE
                } else if e.is::<UnsupportedMediaType>() {
                    StatusCode::UNSUPPORTED_MEDIA_TYPE
                } else if e.is::<::body::BodyReadError>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<::body::BodyDeserializeError>() {
                    StatusCode::BAD_REQUEST
                } else if e.is::<::cors::CorsForbidden>() {
                    StatusCode::FORBIDDEN
                } else if e.is::<::ext::MissingExtension>() {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else if e.is::<::reply::ReplyHttpError>() {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else if e.is::<::reply::ReplyJsonError>() {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else if e.is::<::body::BodyConsumedMultipleTimes>() {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else if e.is::<::fs::FsNeedsTokioThreadpool>() {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else {
                    unreachable!("unexpected 'Known' rejection: {:?}", e);
                }
            }
            Rejections::KnownStatus(status) => status,
            Rejections::With(ref rej, _) => rej.status(),
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
            Rejections::KnownStatus(ref s) => {
                use reply::Reply;
                s.into_response()
            }
            Rejections::With(ref rej, ref e) => {
                let mut res = rej.into_response();

                let bytes = e.to_string();
                res.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                *res.body_mut() = Body::from(bytes);

                res
            }
            Rejections::Custom(ref e) => {
                error!(
                    "unhandled custom rejection, returning 500 response: {:?}",
                    e
                );
                let body = format!("Unhandled rejection: {}", e);
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

    fn cause(&self) -> Option<&Cause> {
        match *self {
            Rejections::Known(ref e) => Some(e),
            Rejections::KnownStatus(_) => None,
            Rejections::With(_, ref e) => Some(e),
            Rejections::Custom(ref e) => Some(e),
            Rejections::Combined(ref a, ref b) => preferred(a, b).cause(),
        }
    }

    pub fn find_cause<T: StdError + 'static>(&self) -> Option<&T> {
        match *self {
            Rejections::Known(ref e) => e.downcast_ref(),
            Rejections::KnownStatus(_) => None,
            Rejections::With(_, ref e) => e.downcast_ref(),
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
            Rejections::KnownStatus(ref s) => f.debug_tuple("Status").field(s).finish(),
            Rejections::With(ref rej, ref e) => f.debug_tuple("With").field(rej).field(e).finish(),
            Rejections::Custom(ref e) => f.debug_tuple("Custom").field(e).finish(),
            Rejections::Combined(ref a, ref b) => {
                f.debug_tuple("Combined").field(a).field(b).finish()
            }
        }
    }
}

/// Invalid query
#[derive(Debug)]
pub struct InvalidQuery(());

impl ::std::fmt::Display for InvalidQuery {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.write_str("Invalid query string")
    }
}

impl StdError for InvalidQuery {
    fn description(&self) -> &str {
        "Invalid query string"
    }
}

/// HTTP method not allowed
#[derive(Debug)]
pub struct MethodNotAllowed(());

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

/// A content-length header is required
#[derive(Debug)]
pub struct LengthRequired(());

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

/// The request payload is too large
#[derive(Debug)]
pub struct PayloadTooLarge(());

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

/// The request's content-type is not supported
#[derive(Debug)]
pub struct UnsupportedMediaType(());

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

/// Missing request header
#[derive(Debug)]
pub struct MissingHeader(&'static str);

impl ::std::fmt::Display for MissingHeader {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Missing request header '{}'", self.0)
    }
}

impl StdError for MissingHeader {
    fn description(&self) -> &str {
        "Missing request header"
    }
}

/// Invalid request header
#[derive(Debug)]
pub struct InvalidHeader(&'static str);

impl ::std::fmt::Display for InvalidHeader {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Invalid request header '{}'", self.0)
    }
}

impl StdError for InvalidHeader {
    fn description(&self) -> &str {
        "Invalid request header"
    }
}


/// Missing cookie
#[derive(Debug)]
pub struct MissingCookie(&'static str);

impl ::std::fmt::Display for MissingCookie {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Missing request cookie '{}'", self.0)
    }
}

impl StdError for MissingCookie {
    fn description(&self) -> &str {
        "Missing request cookie"
    }
}

trait Typed: StdError + 'static {
    fn type_id(&self) -> ::std::any::TypeId;
}

mod sealed {
    use super::{Cause, Reason, Rejection, Rejections};
    use http::StatusCode;
    use never::Never;
    use std::fmt;

    pub trait Reject: fmt::Debug + Send + Sync {
        fn status(&self) -> StatusCode;
        fn into_response(&self) -> ::reply::Response;
        fn cause(&self) -> Option<&Cause> {
            None
        }
    }

    fn _assert_object_safe() {
        fn _assert(_: &dyn Reject) {}
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
    use http::header::CONTENT_TYPE;

    use super::*;
    use http::StatusCode;

    #[allow(deprecated)]
    #[test]
    fn rejection_status() {
        assert_eq!(bad_request().status(), StatusCode::BAD_REQUEST);
        assert_eq!(forbidden().status(), StatusCode::FORBIDDEN);
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
        assert_eq!(server_error().status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(custom("boom").status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[allow(deprecated)]
    #[test]
    fn combine_rejections() {
        let left = bad_request().with("left");
        let right = server_error().with("right");
        let reject = left.combine(right);

        assert_eq!(reject.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(reject.cause().unwrap().to_string(), "right");
    }

    #[allow(deprecated)]
    #[test]
    fn combine_rejection_causes_with_some_left_and_none_server_error() {
        let left = bad_request().with("left");
        let right = server_error();
        let reject = left.combine(right);

        assert_eq!(reject.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(reject.cause().is_none());
    }

    #[allow(deprecated)]
    #[test]
    fn combine_rejection_causes_with_some_left_and_none_right() {
        let left = bad_request().with("left");
        let right = bad_request();
        let reject = left.combine(right);

        assert_eq!(reject.status(), StatusCode::BAD_REQUEST);
        assert_eq!(reject.cause().unwrap().to_string(), "left");
    }

    #[allow(deprecated)]
    #[test]
    fn combine_rejection_causes_with_none_left_and_some_right() {
        let left = bad_request();
        let right = server_error().with("right");
        let reject = left.combine(right);

        assert_eq!(reject.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(reject.cause().unwrap().to_string(), "right");
    }

    #[allow(deprecated)]
    #[test]
    fn unhandled_customs() {
        let reject = bad_request().combine(custom("right"));

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: right");

        // There's no real way to determine which is worse, since both are a 500,
        // so pick the first one.
        let reject = server_error().combine(custom("right"));

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "");

        // With many rejections, custom still is top priority.
        let reject = bad_request()
            .combine(bad_request())
            .combine(not_found())
            .combine(custom("right"))
            .combine(bad_request());

        let resp = reject.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response_body_string(resp), "Unhandled rejection: right");
    }

    #[test]
    fn into_response_with_none_cause() {
        let resp = not_found().into_response();
        assert_eq!(404, resp.status());
        assert!(resp.headers().get(CONTENT_TYPE).is_none());
        assert_eq!("", response_body_string(resp))
    }

    #[allow(deprecated)]
    #[test]
    fn into_response_with_some_cause() {
        let resp = server_error().with("boom").into_response();
        assert_eq!(500, resp.status());
        assert_eq!(
            "text/plain; charset=utf-8",
            resp.headers().get(CONTENT_TYPE).unwrap()
        );
        assert_eq!("boom", response_body_string(resp))
    }

    #[allow(deprecated)]
    #[test]
    fn into_json_with_none_cause() {
        let resp = not_found().json();
        assert_eq!(404, resp.status());
        assert_eq!(
            "application/json",
            resp.headers().get(CONTENT_TYPE).unwrap()
        );
        assert_eq!("{}", response_body_string(resp))
    }

    #[allow(deprecated)]
    #[test]
    fn into_json_with_some_cause() {
        let resp = bad_request().with("boom").json();
        assert_eq!(400, resp.status());
        assert_eq!(
            "application/json",
            resp.headers().get(CONTENT_TYPE).unwrap()
        );
        let expected = "{\"description\":\"boom\",\"message\":\"boom\"}";
        assert_eq!(expected, response_body_string(resp))
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
    #[allow(deprecated)]
    fn into_cause() {
        use std::io;

        let reject = bad_request().with(io::Error::new(io::ErrorKind::Other, "boom"));

        reject.into_cause::<io::Error>().unwrap_err();
    }

    #[allow(deprecated)]
    #[test]
    fn find_cause() {
        use std::io;

        let rej = bad_request().with(io::Error::new(io::ErrorKind::Other, "boom"));

        assert_eq!(rej.find_cause::<io::Error>().unwrap().to_string(), "boom");

        let rej = bad_request()
            .with(io::Error::new(io::ErrorKind::Other, "boom"))
            .combine(method_not_allowed());

        assert_eq!(rej.find_cause::<io::Error>().unwrap().to_string(), "boom");
        assert!(
            rej.find_cause::<MethodNotAllowed>().is_some(),
            "MethodNotAllowed"
        );
    }

    #[test]
    fn size_of_rejection() {
        assert_eq!(
            ::std::mem::size_of::<Rejection>(),
            ::std::mem::size_of::<usize>(),
        );
    }
}
