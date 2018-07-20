//! Body filters
//!
//! Filters that extract a body for a route.

use bytes::Buf;
use futures::{Async, Future, Poll, Stream};
use futures::stream::Concat2;
use http::header::CONTENT_TYPE;
use hyper::{Body, Chunk};
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use ::never::Never;
use ::filter::{Filter, filter_fn, filter_fn_one, One};
use ::reject::{self, Rejection};

/// Extracts the `Body` Stream from the route.
///
/// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract=One<Body>, Error=Never> + Copy {
    filter_fn_one(|route| {
        Ok::<_, Never>(route.take_body())
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> impl Filter<Extract=One<FullBody>, Error=Rejection> + Copy {
    filter_fn_one(move |route| {
        let body = route.take_body();
        Concat {
            fut: body.concat2(),
        }
    })
}

// Require the `content-type` header to be this type (or, if there's no `content-type`
// header at all, optimistically hope it's the right type).
fn is_content_type(ct: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Copy {
    filter_fn(move |route| {
        if let Some(value) = route.headers().get(CONTENT_TYPE) {
            trace!("is_content_type {:?}? {:?}", ct, value);
            if value == ct {
                Ok(())
            } else {
                debug!("content-type doesn't match {:?}", ct);
                Err(reject::unsupported_media_type())
            }
        } else {
            // Optimistically assume its correct!
            trace!("no content-type header, assuming {:?}", ct);
            Ok(())
        }
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a JSON-decoded body.
pub fn json<T: DeserializeOwned + Send>() -> impl Filter<Extract=One<T>, Error=Rejection> + Copy {
    is_content_type("application/json")
        .and(concat())
        .and_then(|buf: FullBody| {
            serde_json::from_slice(&buf.chunk)
                .map_err(|err| {
                    debug!("request json body error: {}", err);
                    reject::bad_request()
                })
        })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
pub fn form<T: DeserializeOwned + Send>() -> impl Filter<Extract=One<T>, Error=Rejection> + Copy {
    is_content_type("application/x-www-form-urlencoded")
        .and(concat())
        .and_then(|buf: FullBody| {
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

