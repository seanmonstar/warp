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

use ::filter::{FilterBase, Filter, filter_fn, filter_fn_one};
use ::reject::{self, Rejection};

use self::sealed::ImplStream;

// Extracts the `Body` Stream from the route.
//
// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract=(Body,), Error=Rejection> + Copy {
    filter_fn_one(|route| {
        route
            .take_body()
            .map(Ok)
            .unwrap_or_else(|| {
                warn!("request body already taken in previous filter");
                Err(reject::server_error())
            })
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

/// Create a `Filter` that extracts the request body as a `futures::Stream`.
///
/// If other filters have already extracted the body, this filter will reject
/// with a `500 Internal Server Error`.
///
/// # Note
///
/// The `ImplStream` type is essentially
/// `impl Stream<Item = impl Buf, Error = warp::Error>`, but since nested
/// `impl Trait`s aren't valid yet, the type acts as one.
pub fn stream() -> impl Filter<Extract=(ImplStream,), Error=Rejection> + Copy {
    body().map(|body: Body| ImplStream {
        body,
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a concatenated body.
pub fn concat() -> impl Filter<Extract=(FullBody,), Error=Rejection> + Copy {
    body().and_then(|body: ::hyper::Body| {
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

mod sealed {
    use bytes::Buf;
    use futures::{Poll, Stream};
    use hyper::{Body, Chunk};

    // It'd be preferable if `warp::body::stream()` could return
    // an `impl Filter<Extract = impl Stream>`, but nested impl Traits
    // aren't yet legal. So, this pretends to be one, by implementing
    // the necessary traits, but not being nameable outside of warp.
    #[derive(Debug)]
    pub struct ImplStream {
        pub(super) body: Body,
    }

    #[derive(Debug)]
    pub struct ImplBuf {
        chunk: Chunk,
    }

    impl Stream for ImplStream {
        type Item = ImplBuf;
        type Error = ::Error;

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            let opt_item = try_ready!(self
                .body
                .poll()
                .map_err(|e| ::Error::from(::error::Kind::Hyper(e)))
            );

            Ok(opt_item.map(|chunk| ImplBuf { chunk }).into())
        }
    }

    impl Buf for ImplBuf {
        fn remaining(&self) -> usize {
            self.chunk.remaining()
        }

        fn bytes(&self) -> &[u8] {
            self.chunk.bytes()
        }

        fn advance(&mut self, cnt: usize) {
            self.chunk.advance(cnt);
        }
    }
}

