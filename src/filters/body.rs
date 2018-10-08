//! Body filters
//!
//! Filters that extract a body for a route.

use std::fmt;

use bytes::Buf;
use futures::{Async, Future, Poll, Stream};
use futures::stream::Concat2;
use headers::ContentLength;
use http::header::CONTENT_TYPE;
use hyper::{Body, Chunk};
use mime;
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use ::filter::{FilterBase, Filter, filter_fn, filter_fn_one};
use ::reject::{self, Rejection};

// Extracts the `Body` Stream from the route.
//
// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract=(Body,), Error=Rejection> + Copy {
    filter_fn_one(|route| {
        route
            .take_body()
            .map(Ok)
            .unwrap_or_else(|| {
                let err = "request body already taken in previous filter";
                error!("{}", err);
                Err(reject::server_error().with(err))
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
    ::filters::header::header2()
        .map_err(|_| {
            debug!("content-length missing");
            reject::length_required()
        })
        .and_then(move |ContentLength(length)| {
            if length <= limit {
                Ok(())
            } else {
                let err = format!("content-length: {} is over limit {}", length, limit);
                debug!("{}", err);
                Err(reject::payload_too_large().with(err))
            }
        })
        .unit()
}

/// Create a `Filter` that extracts the request body as a `futures::Stream`.
///
/// If other filters have already extracted the body, this filter will reject
/// with a `500 Internal Server Error`.
pub fn stream() -> impl Filter<Extract=(BodyStream,), Error=Rejection> + Copy {
    body().map(|body: Body| BodyStream {
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
fn is_content_type(type_: mime::Name<'static>, subtype: mime::Name<'static>)
    -> impl Filter<Extract=(), Error=Rejection> + Copy
{
    filter_fn(move |route| {
        if let Some(value) = route.headers().get(CONTENT_TYPE) {
            trace!("is_content_type {}/{}? {:?}", type_, subtype, value);
            let ct = value.to_str().ok()
                .and_then(|s| s.parse::<mime::Mime>().ok());
            if let Some(ct) = ct {
                if ct.type_() == type_ && ct.subtype() == subtype {
                    Ok(())
                } else {
                    debug!("content-type {:?} doesn't match {}/{}", value, type_, subtype);
                    Err(reject::unsupported_media_type())
                }
            } else {
                debug!("content-type {:?} couldn't be parsed", value);
                Err(reject::unsupported_media_type())
            }
        } else {
            // Optimistically assume its correct!
            trace!("no content-type header, assuming {}/{}", type_, subtype);
            Ok(())
        }
    })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a JSON-decoded body.
pub fn json<T: DeserializeOwned + Send>() -> impl Filter<Extract=(T,), Error=Rejection> + Copy {
    is_content_type(mime::APPLICATION, mime::JSON)
        .and(concat())
        .and_then(|buf: FullBody| {
            serde_json::from_slice(&buf.chunk)
                .map_err(|err| {
                    debug!("request json body error: {}", err);
                    reject::bad_request().with(err)
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
    is_content_type(mime::APPLICATION, mime::WWW_FORM_URLENCODED)
        .and(concat())
        .and_then(|buf: FullBody| {
            serde_urlencoded::from_bytes(&buf.chunk)
                .map_err(|err| {
                    debug!("request form body error: {}", err);
                    reject::bad_request().with(err)
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
            Err(err) => {
                debug!("concat error: {}", err);
                Err(reject::bad_request().with(err))
            }
        }
    }
}

/// An `impl Stream` representing the request body.
///
/// Extracted via the `warp::body::stream` filter.
pub struct BodyStream {
    body: Body,
}

impl Stream for BodyStream {
    type Item = StreamBuf;
    type Error = ::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let opt_item = try_ready!(self
            .body
            .poll()
            .map_err(|e| ::Error::from(::error::Kind::Hyper(e)))
        );

        Ok(opt_item.map(|chunk| StreamBuf { chunk }).into())
    }
}

impl fmt::Debug for BodyStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BodyStream")
            .finish()
    }
}

/// An `impl Buf` representing a chunk in a request body.
///
/// Yielded by a `BodyStream`.
pub struct StreamBuf {
    chunk: Chunk,
}

impl Buf for StreamBuf {
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

impl fmt::Debug for StreamBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.chunk, f)
    }
}

