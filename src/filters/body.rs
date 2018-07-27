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
use ::filter::{FilterBase, Filter, filter_fn, filter_fn_one};
use ::reject::{self, Rejection};

/// Extracts the `Body` Stream from the route.
///
/// Does not consume any of it.
// XXX: Before making public, error should be changeed to Rejection, as it's
// likely that a server error rejection should be returned if trying to take
// the body more than once.
pub(crate) fn body() -> impl Filter<Extract=(Body,), Error=Never> + Copy {
    filter_fn_one(|route| {
        Ok::<_, Never>(route.take_body())
    })
}

/// Require a `content-length` header to have a value no greater than some limit.
///
/// Rejects if `content-length` header is missing, is invalid, or has a number
/// larger than the limit provided.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Limit the upload to 4kb...
/// let upload = warp::body::content_length_limit(4096)
///     .and(warp::body::concat());
/// ```
pub fn content_length_limit(limit: u64) -> impl Filter<Extract=(), Error=Rejection> + Copy {
    ::filters::header::header("content-length")
        .map_err(|_| {
            debug!("content-length missing");
            reject::length_required()
        })
        .and_then(move |length: u64| {
            if length <= limit {
                Ok(())
            } else {
                debug!("content-length: {} is over limit {}", length, limit);
                Err(reject::payload_too_large())
            }
        })
        .unit()
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> impl Filter<Extract=(FullBody,), Error=Rejection> + Copy {
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
pub fn json<T: DeserializeOwned + Send>() -> impl Filter<Extract=(T,), Error=Rejection> + Copy {
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
///
/// # Note
///
/// This filter is for the simpler `application/x-www-form-urlencoded` format,
/// not `multipart/form-data`.
pub fn form<T: DeserializeOwned + Send>() -> impl Filter<Extract=(T,), Error=Rejection> + Copy {
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

