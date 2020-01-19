//! Body filters
//!
//! Filters that extract a body for a route.

use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{buf::BufExt, Buf, Bytes};
use futures::{future, ready, Stream, TryFutureExt};
use headers::ContentLength;
use http::header::CONTENT_TYPE;
use hyper::Body;
use mime;
use serde::de::DeserializeOwned;
use serde_json;
use serde_urlencoded;

use crate::filter::{filter_fn, filter_fn_one, Filter, FilterBase, WrapSealed};
use crate::reject::{self, IsReject, Rejection};
use crate::reply::Reply;

/// Create a wrapping filter to manipulate a request body before it is parsed
///
/// This provides access to the underlying hyper::Body
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let body = warp::body::map_request_body(move |body| {
///     async move {
///         // process the body
///         Some(body)
///     }
/// });
///
/// let route = warp::any()
///     .map(warp::reply)
///     .with(body);
pub fn map_request_body<F, FNOut>(func: F) -> MapRequestBody<F>
where
    F: Fn(Body) -> FNOut + Send + Clone,
    FNOut: std::future::Future<Output = Option<Body>> + Send + 'static,
{
    MapRequestBody { func }
}

/// Decorates a [`Filter`](::Filter) to manipulate bodies
#[derive(Clone, Copy, Debug)]
pub struct MapRequestBody<F> {
    func: F,
}

impl<FN, FNOut, F> WrapSealed<F> for MapRequestBody<FN>
where
    FN: Fn(Body) -> FNOut + Send + Clone,
    FNOut: std::future::Future<Output = Option<Body>> + Send + 'static,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Wrapped = internal::WithBody<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        internal::WithBody {
            filter,
            body: self.clone(),
        }
    }
}

mod internal {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::{ready, TryFuture};
    use hyper::Body;
    use pin_project::pin_project;

    use super::MapRequestBody;
    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::{Reply, Response};
    use crate::route;

    #[allow(missing_debug_implementations)]
    pub struct Bodied(pub(super) Response);

    impl Reply for Bodied {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithBody<FN, F> {
        pub(super) filter: F,
        pub(super) body: MapRequestBody<FN>,
    }

    impl<FN, FNOut, F> FilterBase for WithBody<FN, F>
    where
        FN: Fn(Body) -> FNOut + Clone + Send,
        FNOut: Future<Output = Option<Body>> + Send + 'static,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Extract = (Bodied,);
        type Error = F::Error;
        type Future = WithBodyFuture<Pin<Box<dyn Future<Output = Option<Body>> + Send>>, F::Future>;

        fn filter(&self, _: Internal) -> Self::Future {
            route::with(|route| {
                if let Some(body) = route.take_body() {
                    WithBodyFuture {
                        body: Box::pin((self.body.func)(body))
                            as Pin<Box<dyn Future<Output = Option<Body>> + Send>>,
                        future: self.filter.filter(Internal),
                    }
                } else {
                    WithBodyFuture {
                        body: Box::pin(futures::future::ready(None))
                            as Pin<Box<dyn Future<Output = Option<Body>> + Send>>,
                        future: self.filter.filter(Internal),
                    }
                }
            })
        }
    }

    #[allow(missing_debug_implementations)]
    #[pin_project]
    pub struct WithBodyFuture<F1, F2> {
        #[pin]
        body: F1,
        #[pin]
        future: F2,
    }

    impl<F1, F2> Future for WithBodyFuture<F1, F2>
    where
        F1: Future<Output = Option<Body>> + Send,
        F2: TryFuture,
        F2::Ok: Reply,
        F2::Error: IsReject,
    {
        type Output = Result<(Bodied,), F2::Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let pin = self.as_mut().project();

            match ready!(pin.body.poll(cx)) {
                Some(body) => {
                    route::with(|route| {
                        route.set_body(body);
                    });
                }
                None => (),
            }

            let (result, _status) = match ready!(pin.future.try_poll(cx)) {
                Ok(reply) => {
                    let resp = reply.into_response();
                    let status = resp.status();
                    (Poll::Ready(Ok((Bodied(resp),))), status)
                }
                Err(reject) => {
                    let status = reject.status();
                    (Poll::Ready(Err(reject)), status)
                }
            };

            result
        }
    }
}

// Extracts the `Body` Stream from the route.
//
// Does not consume any of it.
pub(crate) fn body() -> impl Filter<Extract = (Body,), Error = Rejection> + Copy {
    filter_fn_one(|route| {
        future::ready(route.take_body().ok_or_else(|| {
            log::error!("request body already taken in previous filter");
            reject::known(BodyConsumedMultipleTimes { _p: () })
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
///     .and(warp::body::aggregate());
/// ```
pub fn content_length_limit(limit: u64) -> impl Filter<Extract = (), Error = Rejection> + Copy {
    crate::filters::header::header2()
        .map_err(crate::filter::Internal, |_| {
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
pub fn stream(
) -> impl Filter<Extract = (impl Stream<Item = Result<impl Buf, crate::Error>>,), Error = Rejection> + Copy
{
    body().map(|body: Body| BodyStream { body })
}

/// Returns a `Filter` that matches any request and extracts a `Future` of a
/// concatenated body.
///
/// The contents of the body will be flattened into a single contiguous
/// `Bytes`, which may require memory copies. If you don't require a
/// contiguous buffer, using `aggregate` can be give better performance.
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
///     .and(warp::body::bytes())
///     .map(|bytes: bytes::Bytes| {
///         println!("bytes = {:?}", bytes);
///     });
/// ```
pub fn bytes() -> impl Filter<Extract = (Bytes,), Error = Rejection> + Copy {
    body().and_then(|body: hyper::Body| {
        hyper::body::to_bytes(body).map_err(|err| {
            log::debug!("to_bytes error: {}", err);
            reject::known(BodyReadError(err))
        })
    })
}

/// Returns a `Filter` that matches any request and extracts a `Future` of an
/// aggregated body.
///
/// The `Buf` may contain multiple, non-contiguous buffers. This can be more
/// performant (by reducing copies) when receiving large bodies.
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
/// fn full_body(mut body: impl Buf) {
///     // It could have several non-contiguous slices of memory...
///     while body.has_remaining() {
///         println!("slice = {:?}", body.bytes());
///         let cnt = body.bytes().len();
///         body.advance(cnt);
///     }
/// }
///
/// let route = warp::body::content_length_limit(1024 * 32)
///     .and(warp::body::aggregate())
///     .map(full_body);
/// ```
pub fn aggregate() -> impl Filter<Extract = (impl Buf,), Error = Rejection> + Copy {
    body().and_then(|body: ::hyper::Body| {
        hyper::body::aggregate(body).map_err(|err| {
            log::debug!("aggregate error: {}", err);
            reject::known(BodyReadError(err))
        })
    })
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
    async fn from_reader<T: DeserializeOwned + Send>(buf: impl Buf) -> Result<T, Rejection> {
        serde_json::from_reader(buf.reader()).map_err(|err| {
            log::debug!("request json body error: {}", err);
            reject::known(BodyDeserializeError { cause: err.into() })
        })
    }

    is_content_type(mime::APPLICATION, mime::JSON)
        .and(aggregate())
        .and_then(from_reader)
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
    async fn from_reader<T: DeserializeOwned + Send>(buf: impl Buf) -> Result<T, Rejection> {
        serde_urlencoded::from_reader(buf.reader()).map_err(|err| {
            log::debug!("request form body error: {}", err);
            reject::known(BodyDeserializeError { cause: err.into() })
        })
    }

    is_content_type(mime::APPLICATION, mime::WWW_FORM_URLENCODED)
        .and(aggregate())
        .and_then(from_reader)
}

struct BodyStream {
    body: Body,
}

impl Stream for BodyStream {
    type Item = Result<Bytes, crate::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let opt_item = ready!(Pin::new(&mut self.get_mut().body).poll_next(cx));

        match opt_item {
            None => Poll::Ready(None),
            Some(item) => {
                let stream_buf = item.map_err(crate::Error::new);

                Poll::Ready(Some(stream_buf))
            }
        }
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

impl StdError for BodyDeserializeError {}

#[derive(Debug)]
pub(crate) struct BodyReadError(::hyper::Error);

impl ::std::fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Request body read error: {}", self.0)
    }
}

impl StdError for BodyReadError {}

unit_error! {
    pub(crate) BodyConsumedMultipleTimes: "Request body consumed multiple times"
}
