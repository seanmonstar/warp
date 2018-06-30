//! Body filters
//!
//! Filters that extract a body for a route.

use std::marker::PhantomData;

use futures::{Async, Future, Poll, Stream};
use futures::stream::Concat2;
use hyper::{Body, Chunk};
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use ::filter::{Cons, Filter, filter_fn_cons};
use ::route;
use ::Error;

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> impl Filter<Extract=Cons<ConcatFut>> + Copy {
    filter_fn_cons(move || {
        route::with(|route| {
            route.take_body()
                .map(|body| ConcatFut {
                    fut: body.concat2(),
                })
        })
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a JSON-decoded body.
pub fn json<T: DeserializeOwned>() -> impl Filter<Extract=Cons<JsonFut<T>>> + Copy {
    concat()
        .map(|concat| JsonFut {
            concat,
            _marker: PhantomData,
        })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
pub fn form<T: DeserializeOwned>() -> impl Filter<Extract=Cons<FormFut<T>>> + Copy {
    concat()
        .map(|concat| FormFut {
            concat,
            _marker: PhantomData,
        })
}

/// dox?
pub struct ConcatFut {
    fut: Concat2<Body>,
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
pub struct JsonFut<T> {
    concat: ConcatFut,
    _marker: PhantomData<fn() -> T>,
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

/// dox?
pub struct FormFut<T> {
    concat: ConcatFut,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Future for FormFut<T>
where
    T: DeserializeOwned,
{
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let buf = try_ready!(self.concat.poll());
        match serde_urlencoded::from_bytes(&buf) {
            Ok(val) => Ok(Async::Ready(val)),
            Err(err) => {
                debug!("request form body error: {}", err);
                Err(Error(()))
            }
        }
    }
}
