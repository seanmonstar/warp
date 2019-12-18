//! Body filters
//!
//! Filters that extract a body for a route.

use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, Buf};
use futures::{future, ready, TryFutureExt, Stream};
use headers::ContentLength;
use http::header::CONTENT_TYPE;
use hyper::Body;
use mime;
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use crate::filter::{filter_fn, filter_fn_one, Filter, FilterBase};
use crate::reject::{self, Rejection};

// Extracts the `Body` Stream from the route.
//
// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract = (Body,), Error = Rejection> + Copy {
    filter_fn_one(|route| {
        future::ready(route.take_body().ok_or_else(|| {
            log::error!("request body already taken in previous filter");
            reject::known(BodyConsumedMultipleTimes(()))
        }))
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
pub fn content_length_limit(limit: u64) -> impl Filter<Extract = (), Error = Rejection> + Copy {
    crate::filters::header::header2()
        .map_err(|_| {
            log::debug!("content-length missing");
            reject::length_required()
        })
        .and_then(move |ContentLength(length)| {
            if length <= limit {
                future::ok(())
            } else {
                log::debug!("content-length: {} is over limit {}", length, limit);
                future::err(reject::payload_too_large())
            }
        })
        .untuple_one()
}

/// Create a `Filter` that extracts the request body as a `futures::Stream`.
///
/// If other filters have already extracted the body, this filter will reject
/// with a `500 Internal Server Error`.
///
/// # Warning
///
/// This does not have a default size limit, it would be wise to use one to
/// prevent a overly large request from using too much memory.
pub fn stream() -> impl Filter<Extract = (BodyStream,), Error = Rejection> + Copy {
    body().map(|body: Body| BodyStream { body })
}

/// Returns a `Filter` that matches any request and extracts a `Future` of a
/// concatenated body.
///
/// # Warning
///
/// This does not have a default size limit, it would be wise to use one to
/// prevent a overly large request from using too much memory.
///
/// # Example
///
/// ```
/// use warp::{Buf, Filter};
///
/// let route = warp::body::content_length_limit(1024 * 32)
///     .and(warp::body::concat())
///     .map(|mut full_body: warp::body::FullBody| {
///         // FullBody is a `Buf`, which could have several non-contiguous
///         // slices of memory...
///         let mut remaining = full_body.remaining();
///         while remaining != 0 {
///             println!("slice = {:?}", full_body.bytes());
///             let cnt = full_body.bytes().len();
///             full_body.advance(cnt);
///             remaining -= cnt;
///         }
///     });
/// ```
pub fn concat() -> impl Filter<Extract = (FullBody,), Error = Rejection> + Copy {
    body().and_then(|body: ::hyper::Body|
        hyper::body::to_bytes(body)
            .map_ok(|bytes| FullBody { bytes })
            .map_err(|err| {
                log::debug!("concat error: {}", err);
                reject::known(BodyReadError(err))
            })
    )
}

// Require the `content-type` header to be this type (or, if there's no `content-type`
// header at all, optimistically hope it's the right type).
fn is_content_type(
    type_: mime::Name<'static>,
    subtype: mime::Name<'static>,
) -> impl Filter<Extract = (), Error = Rejection> + Copy {
    filter_fn(move |route| {
        if let Some(value) = route.headers().get(CONTENT_TYPE) {
            log::trace!("is_content_type {}/{}? {:?}", type_, subtype, value);
            let ct = value
                .to_str()
                .ok()
                .and_then(|s| s.parse::<mime::Mime>().ok());
            if let Some(ct) = ct {
                if ct.type_() == type_ && ct.subtype() == subtype {
                    future::ok(())
                } else {
                    log::debug!(
                        "content-type {:?} doesn't match {}/{}",
                        value,
                        type_,
                        subtype
                    );
                    future::err(reject::unsupported_media_type())
                }
            } else {
                log::debug!("content-type {:?} couldn't be parsed", value);
                future::err(reject::unsupported_media_type())
            }
        } else {
            // Optimistically assume its correct!
            log::trace!("no content-type header, assuming {}/{}", type_, subtype);
            future::ok(())
        }
    })
}

/// Returns a `Filter` that matches any request and extracts a `Future` of a
/// JSON-decoded body.
///
/// # Warning
///
/// This does not have a default size limit, it would be wise to use one to
/// prevent a overly large request from using too much memory.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use warp::Filter;
///
/// let route = warp::body::content_length_limit(1024 * 32)
///     .and(warp::body::json())
///     .map(|simple_map: HashMap<String, String>| {
///         "Got a JSON body!"
///     });
/// ```
pub fn json<T: DeserializeOwned + Send>() -> impl Filter<Extract = (T,), Error = Rejection> + Copy {
    is_content_type(mime::APPLICATION, mime::JSON)
        .and(concat())
        .and_then(|buf: FullBody| {
            future::ready(serde_json::from_slice(&buf.bytes).map_err(|err| {
                log::debug!("request json body error: {}", err);
                reject::known(BodyDeserializeError { cause: err.into() })
            }))
        })
}

/// Returns a `Filter` that matches any request and extracts a
/// `Future` of a form encoded body.
///
/// # Note
///
/// This filter is for the simpler `application/x-www-form-urlencoded` format,
/// not `multipart/form-data`.
///
/// # Warning
///
/// This does not have a default size limit, it would be wise to use one to
/// prevent a overly large request from using too much memory.
///
///
/// ```
/// use std::collections::HashMap;
/// use warp::Filter;
///
/// let route = warp::body::content_length_limit(1024 * 32)
///     .and(warp::body::form())
///     .map(|simple_map: HashMap<String, String>| {
///         "Got a urlencoded body!"
///     });
/// ```
pub fn form<T: DeserializeOwned + Send>() -> impl Filter<Extract = (T,), Error = Rejection> + Copy {
    is_content_type(mime::APPLICATION, mime::WWW_FORM_URLENCODED)
        .and(concat())
        .and_then(|buf: FullBody| {
            future::ready(serde_urlencoded::from_bytes(&buf.bytes).map_err(|err| {
                log::debug!("request form body error: {}", err);
                reject::known(BodyDeserializeError { cause: err.into() })
            }))
        })
}

/// The full contents of a request body.
///
/// Extracted with the [`concat`](concat) filter.
///
/// As this is a `Buf`, it could have several non-contiguous slices of memory.
#[derive(Debug)]
pub struct FullBody {
    // By concealing how a full body (concat()) is represented, this can be
    // improved to be a `Vec<Chunk>` or similar, thus reducing copies required
    // in the common case.
    bytes: Bytes,
}

impl FullBody {
    #[cfg(feature = "multipart")]
    pub(super) fn into_bytes(self) -> Bytes {
        self.bytes
    }
}

impl Buf for FullBody {
    #[inline]
    fn remaining(&self) -> usize {
        self.bytes.remaining()
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        self.bytes.bytes()
    }

    #[inline]
    fn advance(&mut self, cnt: usize) {
        self.bytes.advance(cnt);
    }
}

/// An `impl Stream` representing the request body.
///
/// Extracted via the `warp::body::stream` filter.
pub struct BodyStream {
    body: Body,
}

impl Stream for BodyStream {
    type Item = Result<StreamBuf, crate::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let opt_item: Option<Result<Bytes, hyper::Error>> = ready!(Pin::new(&mut self.get_mut().body).poll_next(cx));

        match opt_item {
            None =>  Poll::Ready(None),
            Some(item) => {
                let stream_buf = item
                    .map_err(crate::Error::new)
                    .map(|chunk| StreamBuf { chunk });

                Poll::Ready(Some(stream_buf))
            }
        }
    }
}

impl fmt::Debug for BodyStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BodyStream").finish()
    }
}

/// An `impl Buf` representing a chunk in a request body.
///
/// Yielded by a `BodyStream`.
pub struct StreamBuf {
    chunk: Bytes,
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

// ===== Rejections =====

/// An error used in rejections when deserializing a request body fails.
#[derive(Debug)]
pub struct BodyDeserializeError {
    cause: Box<dyn StdError + Send + Sync>,
}

impl fmt::Display for BodyDeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Request body deserialize error: {}", self.cause)
    }
}

impl StdError for BodyDeserializeError {
}

#[derive(Debug)]
pub(crate) struct BodyReadError(::hyper::Error);

impl ::std::fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Request body read error: {}", self.0)
    }
}

impl StdError for BodyReadError {
}

#[derive(Debug)]
pub(crate) struct BodyConsumedMultipleTimes(());

impl ::std::fmt::Display for BodyConsumedMultipleTimes {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.write_str("Request body consumed multiple times")
    }
}

impl StdError for BodyConsumedMultipleTimes {
}
