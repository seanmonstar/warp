//! Reply to requests.
//!
//! A [`Reply`](./trait.Reply.html) is a type that can be converted into an HTTP
//! response to be sent to the client. These are typically the successful
//! counterpart to a [rejection](../reject).
//!
//! The functions in this module are helpers for quickly creating a reply.
//! Besides them, you can return a type that implements [`Reply`](./trait.Reply.html). This
//! could be any of the following:
//!
//! - [`http::Response<impl Into<hyper::Body>`](https://docs.rs/http)
//! - `String`
//! - `&'static str`
//! - `http::StatusCode`
//!
//! # Example
//!
//! ```
//! use warp::{Filter, http::Response};
//!
//! // Returns an empty `200 OK` response.
//! let empty_200 = warp::any().map(warp::reply);
//!
//! // Returns a `200 OK` response with custom header and body.
//! let custom = warp::any().map(|| {
//!     Response::builder()
//!         .header("my-custom-header", "some-value")
//!         .body("and a custom body")
//! });
//!
//! // GET requests return the empty 200, POST return the custom.
//! let routes = warp::get2().and(empty_200)
//!     .or(warp::post2().and(custom));
//! ```

use std::error::Error as StdError;
use std::fmt;

use http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use http::{HttpTryFrom, StatusCode};
use hyper::Body;
use serde::Serialize;
use serde_json;


use ::reject::Reject;
// This re-export just looks weird in docs...
#[doc(hidden)]
pub use ::filters::reply as with;
pub(crate) use self::sealed::{Reply_, ReplySealed, ReplyHttpError, Response};

/// Returns an empty `Reply` with status code `200 OK`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // GET /just-ok returns an empty `200 OK`.
/// let route = warp::path("just-ok")
///     .map(|| {
///         println!("got a /just-ok request!");
///         warp::reply()
///     });
/// ```
#[inline]
pub fn reply() -> impl Reply
{
   StatusCode::OK
}

/// Convert the value into a `Reply` with the value encoded as JSON.
///
/// The passed value must implement [`Serialize`][ser]. Many
/// collections do, and custom domain types can have `Serialize` derived.
///
/// [ser]: https://serde.rs
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // GET /ids returns a `200 OK` with a JSON array of ids:
/// // `[1, 3, 7, 13]`
/// let route = warp::path("ids")
///     .map(|| {
///         let our_ids = vec![1, 3, 7, 13];
///         warp::reply::json(&our_ids)
///     });
/// ```
///
/// # Note
///
/// If a type fails to be serialized into JSON, the error is logged at the
/// `error` level, and the returned `impl Reply` will be an empty
/// `500 Internal Server Error` response.
pub fn json<T>(val: &T) -> impl Reply
where
    T: Serialize,
{
    Json {
        inner: serde_json::to_vec(val).map_err(|err| {
            error!("reply::json error: {}", err);
        }),
    }
}

#[allow(missing_debug_implementations)]
struct Json {
    inner: Result<Vec<u8>, ()>,
}

impl ReplySealed for Json {
    #[inline]
    fn into_response(self) -> Response {
        match self.inner {
            Ok(body) => {
                let mut res = Response::new(body.into());
                res.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/json")
                );
                res
            },
            Err(()) => {
                ::reject::known(ReplyJsonError)
                    .into_response()
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct ReplyJsonError;

impl fmt::Display for ReplyJsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl StdError for ReplyJsonError {
    fn description(&self) -> &str {
        "warp::reply::json() failed"
    }
}

/// Reply with a body and `content-type` set to `text/html; charset=utf-8`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let body = r#"
/// <html>
///     <head>
///         <title>HTML with warp!</title>
///     </head>
///     <body>
///         <h1>warp + HTML = :heart:</h1>
///     </body>
/// </html>
/// "#;
///
/// let route = warp::any()
///     .map(|| {
///         warp::reply::html(body)
///     });
/// ```
///
/// # Note
///
/// If a type fails to be serialized into JSON, the error is logged at the
/// `error` level, and the returned `impl Reply` will be an empty
/// `500 Internal Server Error` response.
pub fn html<T>(body: T) -> impl Reply
where
    Body: From<T>,
    T: Send,
{
    Html {
        body,
    }
}

#[allow(missing_debug_implementations)]
struct Html<T> {
    body: T,
}

impl<T> ReplySealed for Html<T>
where
    Body: From<T>,
    T: Send,
{
    #[inline]
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::from(self.body));
        res.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8")
        );
        res
    }
}

/// Types that can be converted into a `Response`.
///
/// This trait is sealed for now (implementations are only allowed inside
/// warp), but it is implemented for the following:
///
/// - `http::StatusCode`
/// - `http::Response<impl Into<hyper::Body>>`
/// - `String`
/// - `&'static str`
//NOTE: This list is duplicated in the module documentation.
pub trait Reply: ReplySealed {
    /* 
    TODO: Currently unsure about having trait methods here, as it
    requires returning an exact type, which I'd rather not commit to.
    Additionally, it doesn't work great with `Box<Reply>`.

    A possible alternative is to have wrappers, like

    - `WithStatus<R: Reply>(StatusCode, R)`


    /// Change the status code of this `Reply`.
    fn with_status(self, status: StatusCode) -> Reply_
    where
        Self: Sized,
    {
        let mut res = self.into_response();
        *res.status_mut() = status;
        Reply_(res)
    }

    /// Add a header to this `Reply`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use warp::Reply;
    ///
    /// let reply = warp::reply()
    ///     .with_header("x-foo", "bar");
    /// ```
    fn with_header<K, V>(self, name: K, value: V) -> Reply_
    where
        Self: Sized,
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        match <HeaderName as HttpTryFrom<K>>::try_from(name) {
            Ok(name) => match <HeaderValue as HttpTryFrom<V>>::try_from(value) {
                Ok(value) => {
                    let mut res = self.into_response();
                    res.headers_mut().append(name, value);
                    Reply_(res)
                },
                Err(err) => {
                    error!("with_header value error: {}", err.into());
                    Reply_(::reject::server_error()
                        .into_response())
                }
            },
            Err(err) => {
                error!("with_header name error: {}", err.into());
                Reply_(::reject::server_error()
                    .into_response())
            }
        }
    }
    */
}

impl<T: ReplySealed> Reply for T {}

fn _assert_object_safe() {
    fn _assert(_: &Reply) {}
}

/// Wrap an `impl Reply` to change its `StatusCode`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any()
///     .map(warp::reply)
///     .map(|reply| {
///         warp::reply::with_status(reply, warp::http::StatusCode::CREATED)
///     });
/// ```
pub fn with_status<T: Reply>(reply: T, status: StatusCode) -> WithStatus<T> {
    WithStatus {
        reply,
        status,
    }
}

/// Wrap an `impl Reply` to change its `StatusCode`.
///
/// Returned by `warp::reply::with_status`.
#[derive(Debug)]
pub struct WithStatus<T> {
    reply: T,
    status: StatusCode,
}

impl<T: Reply> ReplySealed for WithStatus<T> {
    fn into_response(self) -> Response {
        let mut res = self.reply.into_response();
        *res.status_mut() = self.status;
        res
    }
}

/// Wrap an `impl Reply` to add a header when rendering.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any()
///     .map(warp::reply)
///     .map(|reply| {
///         warp::reply::with_header(reply, "server", "warp")
///     });
/// ```
pub fn with_header<T: Reply, K, V>(reply: T, name: K, value: V) -> WithHeader<T>
where
    HeaderName: HttpTryFrom<K>,
    HeaderValue: HttpTryFrom<V>,
{
    let header = match <HeaderName as HttpTryFrom<K>>::try_from(name) {
        Ok(name) => match <HeaderValue as HttpTryFrom<V>>::try_from(value) {
            Ok(value) => {
                Some((name, value))
            },
            Err(err) => {
                error!("with_header value error: {}", err.into());
                None
            }
        },
        Err(err) => {
            error!("with_header name error: {}", err.into());
            None
        }
    };

    WithHeader {
        header,
        reply,
    }
}

/// Wraps an `impl Reply` and adds a header when rendering.
///
/// Returned by `warp::reply::with_header`.
#[derive(Debug)]
pub struct WithHeader<T> {
    header: Option<(HeaderName, HeaderValue)>,
    reply: T,
}

impl<T: Reply> ReplySealed for WithHeader<T> {
    fn into_response(self) -> Response {
        let mut res = self.reply.into_response();
        if let Some((name, value)) = self.header {
            res.headers_mut().insert(name, value);
        }
        res
    }
}

// Seal the `Reply` trait and the `Reply_` wrapper type for now.
mod sealed {
    use hyper::Body;

    use ::generic::{Either, One};
    use ::reject::Reject;

    use super::Reply;

    pub type Response = ::http::Response<Body>;

    // A trait describing the various things that a Warp server can turn into a `Response`.
    pub trait ReplySealed: Send {
        fn into_response(self) -> Response;
    }

    /// ```compile_fail
    /// use warp::Reply;
    ///
    /// let _ = warp::reply().into_response();
    /// ```
    pub fn __warp_replysealed_compilefail_doctest() {
        // Duplicate code to make sure the code is otherwise valid.
        let _ = ::reply().into_response();
    }

    // An opaque type to return `impl Reply` from trait methods.
    #[allow(missing_debug_implementations)]
    pub struct Reply_(pub(crate) Response);

    impl ReplySealed for Reply_ {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    impl<T: Send> ReplySealed for ::http::Response<T>
    where
        Body: From<T>,
    {
        #[inline]
        fn into_response(self) -> Response {
            self.map(Body::from)
        }
    }

    impl ReplySealed for ::http::StatusCode {
        #[inline]
        fn into_response(self) -> Response {
            let mut res = Response::default();
            *res.status_mut() = self;
            res
        }
    }

    impl<T> ReplySealed for Result<T, ::http::Error>
    where
        T: Reply + Send,
    {
        #[inline]
        fn into_response(self) -> Response {
            match self {
                Ok(t) => t.into_response(),
                Err(e) => {
                    error!("reply error: {:?}", e);
                    ::reject::known(ReplyHttpError(e))
                        .into_response()
                }
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct ReplyHttpError(::http::Error);

    impl ::std::fmt::Display for ReplyHttpError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            write!(f, "http::Response::builder error: {}", self.0)
        }
    }

    impl ::std::error::Error for ReplyHttpError {
        fn description(&self) -> &str {
            "http::Response::builder error"
        }
    }

    impl ReplySealed for String {
        #[inline]
        fn into_response(self) -> Response {
            Response::new(Body::from(self))
        }
    }

    impl ReplySealed for &'static str {
        #[inline]
        fn into_response(self) -> Response {
            Response::new(Body::from(self))
        }
    }

    impl<T, U> ReplySealed for Either<T, U>
    where
        T: Reply,
        U: Reply,
    {
        #[inline]
        fn into_response(self) -> Response {
            match self {
                Either::A(a) => a.into_response(),
                Either::B(b) => b.into_response(),
            }
        }
    }

    impl<T> ReplySealed for One<T>
    where
        T: Reply,
    {
        #[inline]
        fn into_response(self) -> Response {
            self.0.into_response()
        }
    }

    impl ReplySealed for ::never::Never {
        #[inline(always)]
        fn into_response(self) -> Response {
            match self {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn json_serde_error() {
        // a HashMap<Vec, _> cannot be serialized to JSON
        let mut map = HashMap::new();
        map.insert(vec![1, 2], 45);

        let res = json(&map).into_response();
        assert_eq!(res.status(), 500);
    }

    #[test]
    fn response_builder_error() {
        let res = ::http::Response::builder()
            .status(1337)
            .body("woops")
            .into_response();

        assert_eq!(res.status(), 500);
    }

}

