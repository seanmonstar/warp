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

use ::filter::Filter;
use ::route::Route;
use ::Error;

pub fn concat() -> Concat {
    Concat {
        _i: (),
    }
}

pub fn json<T: DeserializeOwned>() -> Json<T> {
    Json {
        _marker: PhantomData,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Concat {
    _i: (),
}

pub struct ConcatFut {
    fut: Concat2<Body>,
}

impl Filter for Concat {
    type Extract = ConcatFut;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        route.take_body()
            .map(|(route, body)| (route, ConcatFut {
                fut: body.unwrap().concat2(),
            }))
    }
}

impl Future for ConcatFut {
    type Item = Chunk;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.fut.poll()
            .map_err(|e| unimplemented!("concat error: {}", e))
    }
}

pub struct Json<T> {
    _marker: PhantomData<fn() -> T>,
}

pub struct JsonFut<T> {
    concat: ConcatFut,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Filter for Json<T> {
    type Extract = JsonFut<T>;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        route.take_body()
            .map(|(route, body)| (route, JsonFut {
                concat: ConcatFut {
                    fut: body.unwrap().concat2(),
                },
                _marker: PhantomData,
            }))
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
                trace!("request json body error: {}", err);
                Err(Error(()))
            }
        }
    }
}
