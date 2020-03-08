//! Compression Filters

use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use bytes::Bytes;
use futures::Stream;
use http::header::HeaderValue;
use http::response::{Parts, Response as HttpResponse};
use hyper::{header::CONTENT_ENCODING, Body};
use pin_project::pin_project;

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::filter::{Filter, WrapSealed};
use crate::reject::IsReject;
use crate::reply::{Reply, Response};

use self::internal::WithCompression;

// String constants that are valid `content-encoding` header values
const GZIP: &str = "gzip";
const DEFLATE: &str = "deflate";
const BR: &str = "br";

/// Compression
#[derive(Clone, Copy, Debug)]
pub struct Compression<F> {
    func: F,
}

/// Create a wrapping filter that compresses the Body of a [`Response`](crate::reply::Response)
/// using gzip, adding `content-encoding: gzip` to the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::get()
///     .and(warp::path::end())
///     .and(warp::fs::file("./README.md"))
///     .with(warp::compression::gzip());
/// ```
pub fn gzip() -> Compression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(GzipEncoder::new(props.body));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, HeaderValue::from_static(GZIP));
        Response::from_parts(props.head, body)
    };
    Compression { func }
}

/// Create a wrapping filter that compresses the Body of a [`Response`](crate::reply::Response)
/// using deflate, adding `content-encoding: deflate` to the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::get()
///     .and(warp::path::end())
///     .and(warp::fs::file("./README.md"))
///     .with(warp::compression::deflate());
/// ```
pub fn deflate() -> Compression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(DeflateEncoder::new(props.body));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, HeaderValue::from_static(DEFLATE));
        Response::from_parts(props.head, body)
    };
    Compression { func }
}

/// Create a wrapping filter that compresses the Body of a [`Response`](crate::reply::Response)
/// using brotli, adding `content-encoding: br` to the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::get()
///     .and(warp::path::end())
///     .and(warp::fs::file("./README.md"))
///     .with(warp::compression::brotli());
/// ```
pub fn brotli() -> Compression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(BrotliEncoder::new(props.body));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, HeaderValue::from_static(BR));
        Response::from_parts(props.head, body)
    };
    Compression { func }
}

/// Compression Props
#[derive(Debug)]
pub struct CompressionProps {
    body: CompressableBody,
    head: Parts,
}

impl From<HttpResponse<Body>> for CompressionProps {
    fn from(resp: HttpResponse<Body>) -> Self {
        let (head, body) = resp.into_parts();
        CompressionProps {
            body: body.into(),
            head,
        }
    }
}

/// A wrapper around [`Body`](hyper::Body) that implements [`Stream`](futures::Stream) to be
/// compatible with async_compression's Stream based encoders
#[pin_project]
#[derive(Debug)]
pub struct CompressableBody {
    #[pin]
    body: Body,
}

impl Stream for CompressableBody {
    type Item = std::io::Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use std::io::{Error, ErrorKind};

        let pin = self.project();
        // TODO: Use `.map_err()` (https://github.com/rust-lang/rust/issues/63514) once it is stabilized
        Body::poll_next(pin.body, cx)
            .map(|e| e.map(|res| res.map_err(|_| Error::from(ErrorKind::InvalidData))))
    }
}

impl From<Body> for CompressableBody {
    fn from(body: Body) -> Self {
        CompressableBody { body }
    }
}

impl<FN, F> WrapSealed<F> for Compression<FN>
where
    FN: Fn(CompressionProps) -> Response + Clone + Send,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Wrapped = WithCompression<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithCompression {
            filter,
            compress: self.clone(),
        }
    }
}

mod internal {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::{ready, TryFuture};
    use pin_project::pin_project;

    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::{Reply, Response};
    use crate::route;

    use super::{Compression, CompressionProps};

    #[allow(missing_debug_implementations)]
    pub struct Compressed(pub(super) Response);

    impl Reply for Compressed {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithCompression<FN, F> {
        pub(super) compress: Compression<FN>,
        pub(super) filter: F,
    }

    impl<FN, F> FilterBase for WithCompression<FN, F>
    where
        FN: Fn(CompressionProps) -> Response + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Extract = (Compressed,);
        type Error = F::Error;
        type Future = WithCompressionFuture<FN, F::Future>;

        fn filter(&self, _: Internal) -> Self::Future {
            WithCompressionFuture {
                compress: self.compress.clone(),
                future: self.filter.filter(Internal),
            }
        }
    }

    #[allow(missing_debug_implementations)]
    #[pin_project]
    pub struct WithCompressionFuture<FN, F> {
        compress: Compression<FN>,
        #[pin]
        future: F,
    }

    impl<FN, F> Future for WithCompressionFuture<FN, F>
    where
        FN: Fn(CompressionProps) -> Response,
        F: TryFuture,
        F::Ok: Reply,
        F::Error: IsReject,
    {
        type Output = Result<(Compressed,), F::Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let pin = self.as_mut().project();
            let result = ready!(pin.future.try_poll(cx));
            match result {
                Ok(reply) => {
                    let resp = route::with(|_r| (self.compress.func)(reply.into_response().into()));
                    Poll::Ready(Ok((Compressed(resp),)))
                }
                Err(reject) => Poll::Ready(Err(reject)),
            }
        }
    }
}
