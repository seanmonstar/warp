//! Body filters
//!
//! Filters that extract a body for a route.

use futures::{Future, Poll, Stream};
use futures::stream::Concat2;
use hyper::{Body, Chunk};
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use ::never::Never;
use ::filter::{Cons, Filter, filter_fn_cons};
use ::reject::{self, Rejection};

/// Extracts the `Body` Stream from the route.
///
/// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract=Cons<Body>, Error=Never> + Copy {
    filter_fn_cons(|route| {
        Ok::<_, Never>(route.take_body())
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> impl Filter<Extract=Cons<Chunk>, Error=Rejection> + Copy {
    filter_fn_cons(move |route| {
        let body = route.take_body();
        Concat {
            fut: body.concat2(),
        }
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a JSON-decoded body.
pub fn json<T: DeserializeOwned + Send>() -> impl Filter<Extract=Cons<T>, Error=Rejection> + Copy {
    concat().and_then(|buf: Chunk| {
        serde_json::from_slice(&buf)
            .map_err(|err| {
                debug!("request json body error: {}", err);
                reject::bad_request()
            })
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
pub fn form<T: DeserializeOwned + Send>() -> impl Filter<Extract=Cons<T>, Error=Rejection> + Copy {
    concat().and_then(|buf: Chunk| {
        serde_urlencoded::from_bytes(&buf)
            .map_err(|err| {
                debug!("request form body error: {}", err);
                reject::bad_request()
            })
    })
}

#[allow(missing_debug_implementations)]
struct Concat {
    fut: Concat2<Body>,
}

impl Future for Concat {
    type Item = Chunk;
    type Error = Rejection;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.fut.poll()
            .map_err(|e| {
                debug!("concat error: {}", e);
                reject::bad_request()
            })
    }
}

