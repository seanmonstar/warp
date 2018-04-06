use std::mem;

use futures::{future, Future};
use http;
use hyper::Body;

use ::filter::Either;

pub fn reply<T>(val: T) -> Response
where
    Response: From<T>,
{
    Response::from(val)
}

#[derive(Debug)]
pub struct Response(pub (crate) http::Response<WarpBody>);

impl From<http::Response<WarpBody>> for Response {
    fn from(http: http::Response<WarpBody>) -> Response {
        Response(http)
    }
}

impl From<&'static str> for Response {
    fn from(s: &'static str) -> Response {
        http::Response::builder()
            .header("content-length", &*s.len().to_string())
            .body(WarpBody::wrap(Body::from(s)))
            .unwrap()
            .into()
    }
}

impl From<String> for Response {
    fn from(s: String) -> Response {
        http::Response::builder()
            .header("content-length", &*s.len().to_string())
            .body(WarpBody::wrap(Body::from(s)))
            .unwrap()
            .into()
    }
}

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

pub trait Reply {
    type Future: Future<Item=Response, Error=!>;
    fn into_response(self) -> Self::Future;
}

#[derive(Debug, Default)]
pub struct WarpBody{
    body: Body,
    #[cfg(debug_assertions)]
    route_taken: bool,
}

impl WarpBody {
    pub(crate) fn wrap(body: Body) -> Self {
        WarpBody {
            body,
            #[cfg(debug_assertions)]
            route_taken: false,
        }
    }

    pub(crate) fn unwrap(self) -> Body {
        self.body
    }

    pub(crate) fn route_take(&mut self) -> Self {
        debug_assert!(!self.route_taken);
        #[cfg(debug_assertions)]
        {
            self.route_taken = true;
        }

        WarpBody::wrap(mem::replace(&mut self.body, Body::empty()))
    }
}

impl Reply for Response {
    type Future = future::FutureResult<Response, !>;
    fn into_response(self) -> Self::Future {
        future::ok(self)
    }
}

impl<T: Reply, U: Reply> Reply for Either<T, U> {
    type Future = future::Either<T::Future, U::Future>;
    fn into_response(self) -> Self::Future {
        match self {
            Either::A(a) => future::Either::A(a.into_response()),
            Either::B(b) => future::Either::B(b.into_response()),
        }
    }
}

impl<T> Reply for T
where
    T: Future<Item=Response, Error=!>,
{
    type Future = T;
    fn into_response(self) -> Self::Future {
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NotFound(());

pub const NOT_FOUND: NotFound = NotFound(());

impl Reply for NotFound {
    type Future = future::FutureResult<Response, !>;
    fn into_response(self) -> Self::Future {
            Response(http::Response::builder()
                .status(404)
                .header("content-length", "0")
                .body(WarpBody::wrap(Body::empty()))
                .unwrap())
                .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_route_take() {
        let mut body = WarpBody::wrap(Body::from("test"));
        // A new body has not been taken yet.
        assert!(!body.route_taken);
        // The body has the string 'test'
        assert!(!body.body.is_empty());

        let taken = body.route_take();
        // The taken body itself isn't taken from.
        assert!(!taken.route_taken);
        // The taken body has the 'test' body
        assert!(!taken.body.is_empty());

        // The first body knows it's been taken.
        assert!(body.route_taken);
        assert!(body.body.is_empty());
    }

    #[test]
    #[should_panic]
    fn body_route_take_twice() {
        let mut body = WarpBody::wrap(Body::from("test"));
        let _b1 = body.route_take();
        let _oh_noes = body.route_take();
    }
}

