//! Create responses to reply to requests.

use http;
use http::header::{CONTENT_TYPE, HeaderValue};
use hyper::Body;
use serde::Serialize;
use serde_json;

pub(crate) use self::sealed::{Reply, Reply_, Response};

/// Easily convert a type into a `Response`.
#[inline]
pub fn reply<T>(val: T) -> impl Reply
where
    Reply_: From<T>,
{
    Reply_::from(val)
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

// Seal the `Reply` trait and the `Reply_` wrapper type for now.
mod sealed {
    use hyper::Body;

    use ::filter::{Cons, Either};

    // A trait describing the various things that a Warp server can turn into a `Response`.
    pub trait Reply {
        fn into_response(self) -> Response;
    }

    pub struct Reply_(pub(super) Response);

    impl From<Response> for Reply_ {
        #[inline]
        fn from(r: Response) -> Reply_ {
            Reply_(r)
        }
    }

    impl From<String> for Reply_ {
        #[inline]
        fn from(s: String) -> Reply_ {
            Reply_(Response::new(Body::from(s)))
        }
    }

    impl From<&'static str> for Reply_ {
        #[inline]
        fn from(s: &'static str) -> Reply_ {
            Reply_(Response::new(Body::from(s)))
        }
    }

    impl Reply for Reply_ {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    pub type Response = ::http::Response<Body>;

    impl<T, U> From<Either<T, U>> for Reply_
    where
        Reply_: From<T> + From<U>,
    {
        #[inline]
        fn from(either: Either<T, U>) -> Reply_ {
            match either {
                Either::A(a) => Reply_::from(a),
                Either::B(b) => Reply_::from(b),
            }
        }
    }

    impl<T> From<Cons<T>> for Reply_
    where
        Reply_: From<T>,
    {
        #[inline]
        fn from(cons: Cons<T>) -> Reply_ {
            Reply_::from(cons.0)
        }
    }

    impl<T: Reply, U: Reply> Reply for Either<T, U> {
        #[inline]
        fn into_response(self) -> Response {
            match self {
                Either::A(a) => a.into_response(),
                Either::B(b) => b.into_response(),
            }
        }
    }

    impl<T> Reply for Cons<T>
    where
        T: Reply
    {
        #[inline]
        fn into_response(self) -> Response {
            self.0.into_response()
        }
    }

    impl<T> Reply for ::filter::Extracted<T>
    where
        T: Reply
    {
        #[inline]
        fn into_response(self) -> Response {
            self.item().into_response()
        }
    }

    impl<T> Reply for ::filter::Errored<T>
    where
        T: Reply
    {
        #[inline]
        fn into_response(self) -> Response {
            self.error().into_response()
        }
    }

    impl Reply for ::never::Never {
        #[inline]
        fn into_response(self) -> Response {
            match self {}
        }
    }
}

