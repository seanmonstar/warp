//! dox?

use futures::{future, Future};
use http;
use http::header::{CONTENT_TYPE, HeaderValue};
use hyper::Body;
use serde::Serialize;
use serde_json;

use ::filter::{Cons, Either};
use ::never::Never;

/// Easily convert a type into a `Response`.
#[inline]
pub fn reply<T>(val: T) -> Reply_
where
    Body: From<T>,
{
    Reply_(Response::new(Body::from(val)))
}

/// dox?
pub struct Reply_(Response);

impl Reply for Reply_ {
    type Future = future::FutureResult<Response, Never>;

    fn into_response(self) -> Self::Future {
        future::ok(self.0)
    }
}

/// Easily constructor a 400 Bad Request response.
pub fn client_error() -> Response {
    http::Response::builder()
        .status(400)
        .body(Body::empty())
        .unwrap()
}

/// Convert the value into a `Response` with the value encoded as JSON.
pub fn json<T>(val: T) -> Reply_
where
    T: Serialize,
{
    Reply_(match serde_json::to_string(&val) {
        Ok(s) => {
            let mut res = Response::new(s.into()); //reply(s);
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json")
            );
            res
        },
        Err(e) => {
            debug!("reply::json error: {}", e);
            http::Response::builder()
                .status(500)
                .header("content-length", "0")
                .body(Body::empty())
                .unwrap()
        }
    })
}

/// dox?
pub type Response = http::Response<Body>;
/*
/// An HTTP response used by Warp servers.
#[derive(Debug)]
pub struct Response(pub(crate) http::Response<Body>);

impl From<&'static str> for Response {
    #[inline]
    fn from(s: &'static str) -> Response {
        Response(http::Response::new(Body::from(s)))
    }
}

impl From<String> for Response {
    fn from(s: String) -> Response {
        Response(http::Response::new(Body::from(s)))
    }
}
*/

impl<T, U> From<Either<T, U>> for Response
where
    Response: From<T> + From<U>,
{
    fn from(either: Either<T, U>) -> Response {
        match either {
            Either::A(a) => Response::from(a),
            Either::B(b) => Response::from(b),
        }
    }
}

impl<T> From<Cons<T>> for Response
where
    Response: From<T>,
{
    fn from(cons: Cons<T>) -> Response {
        Response::from(cons.0)
    }
}

/// A trait describing the various things that a Warp server can turn into a `Response`.
pub trait Reply {
    /// The future of the Response.
    type Future: Future<Item=Response, Error=Never> + Send + 'static;
    /// Convert self into `Self::Future`.
    fn into_response(self) -> Self::Future;
}

/*
impl Reply for Response {
    type Future = future::FutureResult<Response, Never>;
    #[inline]
    fn into_response(self) -> Self::Future {
        future::ok(self)
    }
}

impl IntoFuture for Response {
    type Item = Response;
    type Error = Never;
    type Future = future::FutureResult<Response, Never>;
    #[inline]
    fn into_future(self) -> Self::Future {
        future::ok(self)
    }
}
*/

impl<T: Reply, U: Reply> Reply for Either<T, U> {
    type Future = future::Either<T::Future, U::Future>;
    #[inline]
    fn into_response(self) -> Self::Future {
        match self {
            Either::A(a) => future::Either::A(a.into_response()),
            Either::B(b) => future::Either::B(b.into_response()),
        }
    }
}

impl<T> Reply for Cons<T>
where
    T: Reply
{
    type Future = T::Future;
    #[inline]
    fn into_response(self) -> Self::Future {
        self.0.into_response()
    }
}

impl<T, R, E> Reply for T
where
    T: Future<Item=R, Error=E> + Send + 'static,
    R: Reply + 'static,
    E: Reply + 'static,
{
    type Future = future::Then<T, future::Either<R::Future, E::Future>, fn(Result<R, E>) -> future::Either<R::Future, E::Future>>;
    fn into_response(self) -> Self::Future {
        self.then(|result| match result {
            Ok(reply) => future::Either::A(reply.into_response()),
            Err(err) => future::Either::B(err.into_response()),
        })
    }
}

impl Reply for ::Error {
    type Future = future::FutureResult<Response, Never>;
    fn into_response(self) -> Self::Future {
        trace!("NOT_FOUND");
        future::ok(http::Response::builder()
            .status(404)
            .body(Body::empty())
            .unwrap())
    }
}

impl Reply for ::never::Never {
    type Future = future::FutureResult<Response, Never>;
    fn into_response(self) -> Self::Future {
        match self {}
    }
}

