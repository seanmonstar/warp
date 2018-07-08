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
pub fn json<T: DeserializeOwned>() -> impl Filter<Extract=Cons<T>, Error=Rejection> + Copy {
    concat()
        .map(|buf: Chunk| {
            serde_json::from_slice(&buf).expect("unimplemented json error")
            /*
            match serde_json::from_slice(&buf) {
                Ok(val) => Ok(val)
                Err(err) => {
                    debug!("request json body error: {}", err);
                    Err(Error(()))
                }
            }
            */
        })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
pub fn form<T: DeserializeOwned>() -> impl Filter<Extract=Cons<T>, Error=Rejection> + Copy {
    concat()
        .map(|buf: Chunk| {
            serde_urlencoded::from_bytes(&buf).expect("unimplemented form error")
            /*
            match serde_urlencoded::from_bytes(&buf) {
                Ok(val) => Ok(Async::Ready(val)),
                Err(err) => {
                    debug!("request form body error: {}", err);
                    Err(Error(()))
                }
            }
            */
        })
}

/// dox?
pub struct Concat {
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

