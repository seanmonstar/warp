//! Body filters
//!
//! Filters that extract a body for a route.
//!
//! # Body filters must "end" a filter chain
//!
//! ```compile_fail
//! let a = warp::body::concat();
//! let b = warp::body::concat();
//!
//! // Cannot 'and' chain something after 'a'
//! a.and(b)
//! ```

use std::marker::PhantomData;

use futures::{Async, Future, Poll, Stream};
use futures::stream::Concat2;
use hyper::{Body, Chunk};
use serde::de::DeserializeOwned;
use serde_json;

use ::filter::FilterBase;
use ::route;
use ::Error;

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> Concat {
    Concat {
        _i: (),
    }
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a JSON-decoded body.
pub fn json<T: DeserializeOwned>() -> Json<T> {
    Json {
        _marker: PhantomData,
    }
}

/// dox?
#[derive(Clone, Copy, Debug)]
pub struct Concat {
    _i: (),
}

/// dox?
pub struct ConcatFut {
    fut: Concat2<Body>,
}

impl FilterBase for Concat {
    type Extract = ConcatFut;

    fn filter(&self) -> Option<Self::Extract> {
        route::with(|route| {
            route.take_body()
                .map(|body| ConcatFut {
                    fut: body.unwrap().concat2(),
                })
        })
    }
}

impl Future for ConcatFut {
    type Item = Chunk;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.fut.poll()
            .map_err(|e| {
                debug!("concat error: {}", e);
                Error(())
            })
    }
}

/// dox?
pub struct Json<T> {
    _marker: PhantomData<fn() -> T>,
}

/// dox?
pub struct JsonFut<T> {
    concat: ConcatFut,
    _marker: PhantomData<fn() -> T>,
}

impl<T> FilterBase for Json<T> {
    type Extract = JsonFut<T>;

    fn filter(&self) -> Option<Self::Extract> {
        concat()
            .filter()
            .map(|concat| JsonFut {
                concat,
                _marker: PhantomData,
            })
    }
}

impl<T> Future for JsonFut<T>
where
    T: DeserializeOwned,
{
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let buf = try_ready!(self.concat.poll());
        match serde_json::from_slice(&buf) {
            Ok(val) => Ok(Async::Ready(val)),
            Err(err) => {
                debug!("request json body error: {}", err);
                Err(Error(()))
            }
        }
    }
}
