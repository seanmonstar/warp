//! Mismatch
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
//! As a request is processed by a Filter chain, the rejections are accumulated into
//! a list contained by the [`Rejection`](struct.Rejection.html) type. Mismatch from
//! filters can be handled using [`Filter::recover`](../trait.Filter.html#method.recover).
//! This is a convenient way to map rejections into a [`Reply`](../reply/trait.Reply.html).
//!
//! For a more complete example see the
//! [Rejection Example](https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs)
//! from the repository.
//!
//! # Example
//!
//! ```
//! use warp::{reply, Reply, Filter, reject, Rejection, http::StatusCode};
//!
//! #[derive(Debug)]
//! struct InvalidParameter;
//!
//! impl reject::Reject for InvalidParameter {};
//!
//! // Custom rejection handler that maps rejections into responses.
//! async fn handle_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
//!     if err.is_not_found() {
//!         Ok(reply::with_status("NOT_FOUND", StatusCode::NOT_FOUND))
//!     } else if let Some(e) = err.find::<InvalidParameter>() {
//!         Ok(reply::with_status("BAD_REQUEST", StatusCode::BAD_REQUEST))
//!     } else {
//!         eprintln!("unhandled rejection: {:?}", err);
//!         Ok(reply::with_status("INTERNAL_SERVER_ERROR", StatusCode::INTERNAL_SERVER_ERROR))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!
//!     // Filter on `/:id`, but reject with InvalidParameter if the `id` is `0`.
//!     // Recover from this rejection using a custom rejection handler.
//!     let route = warp::path::param()
//!         .and_then(|id: u32| async move {
//!             if id == 0 {
//!                 Err(warp::reject::custom(InvalidParameter))
//!             } else {
//!                 Ok("id is valid")
//!             }
//!         })
//!         .recover(handle_rejection);
//!
//!     warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
//! }
//! ```

use std::any::Any;
use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;

use http::{
    self,
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
};
use hyper::Body;

use crate::Reply;

pub(crate) use self::sealed::{CombineRejection, IsReject};

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn reject() -> Rejection {
    not_found()
}

/// Rejects a request with `404 Not Found`.
#[inline]
pub fn not_found() -> Rejection {
    Rejection {
        reason: Reason::Mismatch(Vec::new()),
    }
}

// 400 Bad Request
#[inline]
pub(crate) fn invalid_query() -> Rejection {
    known(InvalidQuery { _p: () })
}

// 400 Bad Request
#[inline]
pub(crate) fn missing_header(name: &'static str) -> Rejection {
    known(MissingHeader { name })
}

// 400 Bad Request
#[inline]
pub(crate) fn invalid_header(name: &'static str) -> Rejection {
    known(InvalidHeader { name })
}

// 400 Bad Request
#[inline]
pub(crate) fn missing_cookie(name: &'static str) -> Rejection {
    known(MissingCookie { name })
}

// 405 Method Not Allowed
#[inline]
pub(crate) fn method_not_allowed() -> Rejection {
    known(MethodNotAllowed { _p: () })
}

// 411 Length Required
#[inline]
pub(crate) fn length_required() -> Rejection {
    known(LengthRequired { _p: () })
}

// 413 Payload Too Large
#[inline]
pub(crate) fn payload_too_large() -> Rejection {
    known(PayloadTooLarge { _p: () })
}

// 415 Unsupported Media Type
//
// Used by the body filters if the request payload content-type doesn't match
// what can be deserialized.
#[inline]
pub(crate) fn unsupported_media_type() -> Rejection {
    known(UnsupportedMediaType { _p: () })
}

/// Rejects a request with a custom cause.
///
/// A [`recover`][] filter should convert this `Rejection` into a `Reply`,
/// or else this will be returned as a `500 Internal Server Error`.
///
/// [`recover`]: ../trait.Filter.html#method.recover
pub fn custom<T: Reject>(err: T) -> Rejection {
    Rejection::custom(Box::new(err))
}

/// Creates a fatal rejection, which prevents any further matching
///
/// It is converted directly to a Response and sent.
pub fn fatal<T: Reply>(err: T) -> Rejection{
  Rejection::fatal(err)
}

/// Protect against re-rejecting a rejection.
///
/// ```compile_fail
/// fn with(r: warp::Rejection) {
///     let _wat = warp::reject::custom(r);
/// }
/// ```
fn __reject_custom_compilefail() {}

/// A marker trait to ensure proper types are used for custom rejections.
///
/// Can be converted into Rejection.
///
/// # Example
///
/// ```
/// use warp::{Filter, reject::Reject};
///
/// #[derive(Debug)]
/// struct RateLimited;
///
/// impl Reject for RateLimited {}
///
/// let route = warp::any().and_then(|| async {
///     Err::<(), _>(warp::reject::custom(RateLimited))
/// });
/// ```
// Require `Sized` for now to prevent passing a `Box<dyn Reject>`, since we
// would be double-boxing it, and the downcasting wouldn't work as expected.
pub trait Reject: fmt::Debug + Sized + Send + Sync + 'static {}

/// A marker trait to ensure proper types are used for fatal rejections.
///
/// Can be converted into Rejection.
///
/// # Example
///
/// ```
/// use warp::{Filter, reply::{Reply, Response, html, with_status}, http::StatusCode, reject::Fatal};
///
/// #[derive(Debug)]
/// struct DbError;
/// impl Reply for DbError {
///     fn into_response(self) -> Response { with_status(html("DB error"), StatusCode::INTERNAL_SERVER_ERROR).into_response() }
/// }
/// impl Fatal for DbError {}
///
/// let route = warp::any().and_then(|| async {
///     Err::<(), _>(warp::reject::fatal(DbError))
/// });
/// ```
pub trait Fatal: Reply + Send + 'static {}


trait Cause: fmt::Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T> Cause for T
where
    T: fmt::Debug + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl dyn Cause {
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

pub(crate) fn known<T: Into<Known>>(err: T) -> Rejection {
    Rejection::known(err.into())
}

/// Rejection of a request by a [`Filter`](crate::Filter).
///
/// See the [`reject`](module@crate::reject) documentation for more.
#[derive(Debug)]
pub struct Rejection {
    reason: Reason,
}

#[derive(Debug)]
enum Reason {
    Mismatch(Vec<Mismatch>),
    Fatal(Box<crate::reply::Response>),
}

#[derive(Debug)]
enum Mismatch{
    Custom(Box<dyn Cause>),
    Known(Known),
}

macro_rules! enum_known {
     ($($(#[$attr:meta])* $var:ident($ty:path),)+) => (
        pub(crate) enum Known {
            $(
            $(#[$attr])*
            $var($ty),
            )+
        }

        impl Known {
            fn inner_as_any(&self) -> &dyn Any {
                match *self {
                    $(
                    $(#[$attr])*
                    Known::$var(ref t) => t,
                    )+
                }
            }
        }

        impl fmt::Debug for Known {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                    $(#[$attr])*
                    Known::$var(ref t) => t.fmt(f),
                    )+
                }
            }
        }

        impl fmt::Display for Known {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $(
                    $(#[$attr])*
                    Known::$var(ref t) => t.fmt(f),
                    )+
                }
            }
        }

        $(
        #[doc(hidden)]
        $(#[$attr])*
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
    InvalidHeader(InvalidHeader),
    MissingHeader(MissingHeader),
    MissingCookie(MissingCookie),
    InvalidQuery(InvalidQuery),
    LengthRequired(LengthRequired),
    PayloadTooLarge(PayloadTooLarge),
    UnsupportedMediaType(UnsupportedMediaType),
    FileOpenError(crate::fs::FileOpenError),
    FilePermissionError(crate::fs::FilePermissionError),
    BodyReadError(crate::body::BodyReadError),
    BodyDeserializeError(crate::body::BodyDeserializeError),
    CorsForbidden(crate::cors::CorsForbidden),
    #[cfg(feature = "websocket")]
    MissingConnectionUpgrade(crate::ws::MissingConnectionUpgrade),
    MissingExtension(crate::ext::MissingExtension),
    BodyConsumedMultipleTimes(crate::body::BodyConsumedMultipleTimes),
}

impl Rejection {
    fn known(known: Known) -> Self {
        Rejection {
            reason: Reason::Mismatch(vec![Mismatch::Known(known)]),
        }
    }

    fn custom(other: Box<dyn Cause>) -> Self {
        Rejection {
            reason: Reason::Mismatch(vec![Mismatch::Custom(other)]),
        }
    }

    fn fatal<T: Reply>(err: T) -> Self {
        Rejection{
            reason:Reason::Fatal(Box::new(err.into_response())),
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
    /// #[derive(Debug)]
    /// struct Nope;
    ///
    /// impl warp::reject::Reject for Nope {}
    ///
    /// let reject = warp::reject::custom(Nope);
    ///
    /// if let Some(nope) = reject.find::<Nope>() {
    ///    println!("found it: {:?}", nope);
    /// }
    /// ```
    pub fn find<T: 'static>(&self) -> Option<&T> {
        if let Reason::Mismatch(ref mismatches) = self.reason {
            for mismatch in mismatches {
                if let Some(requested) = mismatch.find() {
                    return Some(requested);
                }
            }
        }
        None
    }

    /// Returns true if this Rejection was made via `warp::reject::not_found`.
    ///
    /// # Example
    ///
    /// ```
    /// let rejection = warp::reject();
    ///
    /// assert!(rejection.is_not_found());
    /// ```
    pub fn is_not_found(&self) -> bool {
        match &self.reason {
            Reason::Mismatch(m) => m.is_empty(),
            _ => false,
        }
    }
}

//impl<T: Fatal> From<T> for Rejection {
//    #[inline]
//    fn from(err: T) -> Rejection {
//        fatal(err)
//    }
//}

impl<T: Reject> From<T> for Rejection {
    #[inline]
    fn from(err: T) -> Rejection {
        custom(err)
    }
}

impl From<Infallible> for Rejection {
    #[inline]
    fn from(infallible: Infallible) -> Rejection {
        match infallible {}
    }
}

fn preferred<'a>(mismatches: &'a Vec<Mismatch>) -> Option<&'a Mismatch> {
    // Compare status codes, with this priority:
    // - NOT_FOUND is lowest
    // - METHOD_NOT_ALLOWED is second
    // - if one status code is greater than the other
    // - otherwise, prefer A...
    let mut tmp = None;
    let mut tmp_status = StatusCode::NOT_FOUND;
    for mismatch in mismatches {
        match (tmp_status, Some(mismatch).status()) {
            (_, StatusCode::NOT_FOUND) => (),
            (StatusCode::NOT_FOUND, _) => {
                tmp = Some(mismatch);
                tmp_status = tmp.status();
            },
            (_, StatusCode::METHOD_NOT_ALLOWED) => (),
            (StatusCode::METHOD_NOT_ALLOWED, _) => {
                tmp = Some(mismatch);
                tmp_status = tmp.status();
            },
            (sa, sb) if sa < sb => {
                tmp = Some(mismatch);
                tmp_status = tmp.status();
            },
            _ => (),
        }
    }
    tmp
}


impl IsReject for Infallible {
    fn status(&self) -> StatusCode {
        match *self {}
    }

    fn into_response(self) -> crate::reply::Response {
        match self {}
    }
}

impl IsReject for Rejection {
    fn status(&self) -> StatusCode {
        match self.reason {
            Reason::Fatal(ref response) => response.status(),
            Reason::Mismatch(ref other) => preferred(other).status(),
        }
    }

    fn into_response(self) -> crate::reply::Response {
        match self.reason {
            Reason::Fatal(response) => *response,
            Reason::Mismatch(other) => preferred(&other).into_response(),
        }
    }
}

// ===== Mismatch =====

impl IsReject for Option<&Mismatch> {
    fn status(&self) -> StatusCode {
        match *self {
            Some(inner) => match inner {
                Mismatch::Known(ref k) => match *k {
                    Known::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
                    Known::InvalidHeader(_)
                    | Known::MissingHeader(_)
                    | Known::MissingCookie(_)
                    | Known::InvalidQuery(_)
                    | Known::BodyReadError(_)
                    | Known::BodyDeserializeError(_) => StatusCode::BAD_REQUEST,
                    #[cfg(feature = "websocket")]
                    Known::MissingConnectionUpgrade(_) => StatusCode::BAD_REQUEST,
                    Known::LengthRequired(_) => StatusCode::LENGTH_REQUIRED,
                    Known::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
                    Known::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    Known::FilePermissionError(_) | Known::CorsForbidden(_) => StatusCode::FORBIDDEN,
                    Known::FileOpenError(_)
                    | Known::MissingExtension(_)
                    | Known::BodyConsumedMultipleTimes(_) => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Mismatch::Custom(..) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            None => {
                StatusCode::NOT_FOUND
            },
        }
    }

    fn into_response(self) -> crate::reply::Response {
        match self {
            Some(inner) => match inner {
                Mismatch::Known(ref e) => {
                    let mut res = http::Response::new(Body::from(e.to_string()));
                    *res.status_mut() = self.status();
                    res.headers_mut().insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("text/plain; charset=utf-8"),
                    );
                    res
                }
                Mismatch::Custom(ref e) => {
                    tracing::error!(
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
            },
            None => {
                let mut response = http::Response::new(Body::from("Not found"));
                *response.status_mut() = self.status();
                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                response
            },
        }
    }
}
impl Mismatch {
    fn find<T: 'static>(&self) -> Option<&T> {
        match *self {
            Mismatch::Known(ref e) => e.inner_as_any().downcast_ref(),
            Mismatch::Custom(ref e) => e.downcast_ref(),
        }
    }
}

unit_error! {
    /// Invalid query
    pub InvalidQuery: "Invalid query string"
}
unit_error! {
    /// HTTP method not allowed
    pub MethodNotAllowed: "HTTP method not allowed"
}
unit_error! {
    /// A content-length header is required
    pub LengthRequired: "A content-length header is required"
}
unit_error! {
    /// The request payload is too large
    pub PayloadTooLarge: "The request payload is too large"
}
unit_error! {
    /// The request's content-type is not supported
    pub UnsupportedMediaType: "The request's content-type is not supported"
}

/// Missing request header
#[derive(Debug)]
pub struct MissingHeader {
    name: &'static str,
}
impl MissingHeader {
    /// Retrieve the name of the header that was missing
    pub fn name(&self) -> &str {
        self.name
    }
}
impl ::std::fmt::Display for MissingHeader {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Missing request header {:?}", self.name)
    }
}
impl StdError for MissingHeader {}

/// Invalid request header
#[derive(Debug)]
pub struct InvalidHeader {
    name: &'static str,
}
impl InvalidHeader {
    /// Retrieve the name of the header that was invalid
    pub fn name(&self) -> &str {
        self.name
    }
}
impl ::std::fmt::Display for InvalidHeader {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Invalid request header {:?}", self.name)
    }
}
impl StdError for InvalidHeader {}

/// Missing cookie
#[derive(Debug)]
pub struct MissingCookie {
    name: &'static str,
}
impl MissingCookie {
    /// Retrieve the name of the cookie that was missing
    pub fn name(&self) -> &str {
        self.name
    }
}
impl ::std::fmt::Display for MissingCookie {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Missing request cookie {:?}", self.name)
    }
}
impl StdError for MissingCookie {}

mod sealed;

#[cfg(test)]
mod tests;
