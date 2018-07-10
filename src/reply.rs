//! Create responses to reply to requests.

use http::header::{CONTENT_TYPE, HeaderValue};
use serde::Serialize;
use serde_json;

pub use http::StatusCode;

pub(crate) use self::sealed::{ReplySealed, Response};

/// Easily convert a type into a `Response`.
#[inline]
pub fn reply(val: impl Reply) -> impl Reply
{
    val
}

/// Convert the value into a `Response` with the value encoded as JSON.
pub fn json<T>(val: &T) -> impl Reply
where
    T: Serialize,
{
    match serde_json::to_string(val) {
        Ok(s) => {
            let mut res = Response::new(s.into());
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json")
            );
            res
        },
        Err(e) => {
            use ::reject::Reject;
            debug!("reply::json error: {}", e);
            ::reject::server_error()
                .into_response()
        }
    }
}

/// A trait describing the various things that a Warp server can turn into a `Response`.
pub trait Reply: ReplySealed {
}

impl<T: ReplySealed> Reply for T {}

// Seal the `Reply` trait and the `Reply_` wrapper type for now.
mod sealed {
    use hyper::Body;

    use ::filter::{Cons, Either};

    use super::Reply;

    pub type Response = ::http::Response<Body>;

    // A trait describing the various things that a Warp server can turn into a `Response`.
    pub trait ReplySealed {
        fn into_response(self) -> Response;
    }

    /*
    pub struct Reply_(pub(super) Response);

    impl ReplySealed for Reply_ {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }
    */

    impl ReplySealed for Response {
        #[inline]
        fn into_response(self) -> Response {
            self
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

    impl<T> ReplySealed for Cons<T>
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

