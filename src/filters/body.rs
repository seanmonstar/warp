//! Body filters
//!
//! Filters that extract a body for a route.

use bytes::Buf;
use futures::{Async, Future, Poll, Stream};
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
pub fn concat() -> impl Filter<Extract=Cons<FullBody>, Error=Rejection> + Copy {
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
    concat().and_then(|buf: FullBody| {
        serde_json::from_slice(&buf.chunk)
            .map_err(|err| {
                debug!("request json body error: {}", err);
                reject::bad_request()
            })
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
pub fn form<T: DeserializeOwned + Send>() -> impl Filter<Extract=Cons<T>, Error=Rejection> + Copy {
    concat().and_then(|buf: FullBody| {
        serde_urlencoded::from_bytes(&buf.chunk)
            .map_err(|err| {
                debug!("request form body error: {}", err);
                reject::bad_request()
            })
    })
}

/// The full contents of a request body.
///
/// Extracted with the [`concat`](concat) filter.
#[derive(Debug)]
pub struct FullBody {
    // By concealing how a full body (concat()) is represented, this can be
    // improved to be a `Vec<Chunk>` or similar, thus reducing copies required
    // in the common case.
    chunk: Chunk,
}

impl Buf for FullBody {
    #[inline]
    fn remaining(&self) -> usize {
        self.chunk.remaining()
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        self.chunk.bytes()
    }

    #[inline]
    fn advance(&mut self, cnt: usize) {
        self.chunk.advance(cnt);
    }
}

#[allow(missing_debug_implementations)]
struct Concat {
    fut: Concat2<Body>,
}

impl Future for Concat {
    type Item = FullBody;
    type Error = Rejection;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.fut.poll() {
            Ok(Async::Ready(chunk)) => Ok(Async::Ready(FullBody { chunk, })),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => {
                debug!("concat error: {}", e);
                Err(reject::bad_request())
            }
        }
    }
}

