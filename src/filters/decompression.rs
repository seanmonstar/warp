//! Decompression Filters
//!
//! Filters that decompress the body of a request.

#[cfg(feature = "compression-brotli")]
use async_compression::tokio::bufread::BrotliDecoder;

#[cfg(feature = "compression-gzip")]
use async_compression::tokio::bufread::{DeflateDecoder, GzipDecoder};

use http::header::HeaderValue;
use hyper::{
    header::{CONTENT_ENCODING, CONTENT_LENGTH},
    Body,
};
use tokio_util::io::{ReaderStream, StreamReader};

use crate::filter::{Filter, WrapSealed};
use crate::reject::IsReject;
use crate::reply::{Reply, Response};

use self::internal::{CompressionProps, WithDecompression};

enum DecompressionAlgo {
    #[cfg(feature = "compression-brotli")]
    BR,
    #[cfg(feature = "compression-gzip")]
    DEFLATE,
    #[cfg(feature = "compression-gzip")]
    GZIP,
}

impl From<DecompressionAlgo> for HeaderValue {
    #[inline]
    fn from(algo: DecompressionAlgo) -> Self {
        HeaderValue::from_static(match algo {
            #[cfg(feature = "compression-brotli")]
            DecompressionAlgo::BR => "br",
            #[cfg(feature = "compression-gzip")]
            DecompressionAlgo::DEFLATE => "deflate",
            #[cfg(feature = "compression-gzip")]
            DecompressionAlgo::GZIP => "gzip",
        })
    }
}

/// Decompression
#[derive(Clone, Copy, Debug)]
pub struct Decompression<F> {
    func: F,
}

/// Create a wrapping filter that decompresses the Body of a [`Response`](crate::reply::Response)
/// using gzip, removing `content-encoding: gzip` from the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// // use warp::Filter;
///
///  let route = warp::get()
///  .and(warp::path::end())
/// .and(warp::fs::file("./README.md"))
/// .with(warp::decompression::gzip());
///
/// ```
#[cfg(feature = "compression-gzip")]
pub fn deflate() -> Decompression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(ReaderStream::new(DeflateDecoder::new(StreamReader::new(
            props.body,
        ))));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, DecompressionAlgo::DEFLATE.into());
        props.head.headers.remove(CONTENT_LENGTH);
        Response::from_parts(props.head, body)
    };
    Decompression { func }
}

/// Create a wrapping filter that decompresses the Body of a [`Response`](crate::reply::Response)
/// using brotli, removing `content-encoding: br` from the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::get()
/// .and(warp::path::end())
/// .and(warp::fs::file("./README.md"))
/// .with(warp::decompression::brotli());
/// ```
#[cfg(feature = "compression-brotli")]
pub fn brotli() -> Decompression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(ReaderStream::new(BrotliDecoder::new(StreamReader::new(
            props.body,
        ))));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, DecompressionAlgo::BR.into());
        props.head.headers.remove(CONTENT_LENGTH);
        Response::from_parts(props.head, body)
    };
    Decompression { func }
}

/// Create a wrapping filter that decompresses the Body of a [`Response`](crate::reply::Response)
/// using gzip, removing `content-encoding: gzip` from the Response's [`HeaderMap`](hyper::HeaderMap)
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::get()
///     .and(warp::path::end())
///     .and(warp::fs::file("./README.md"))
///     .with(warp::decompression::gzip());
/// ```
#[cfg(feature = "compression-gzip")]
pub fn gzip() -> Decompression<impl Fn(CompressionProps) -> Response + Copy> {
    let func = move |mut props: CompressionProps| {
        let body = Body::wrap_stream(ReaderStream::new(GzipDecoder::new(StreamReader::new(
            props.body,
        ))));
        props
            .head
            .headers
            .append(CONTENT_ENCODING, DecompressionAlgo::GZIP.into());
        props.head.headers.remove(CONTENT_LENGTH);
        Response::from_parts(props.head, body)
    };
    Decompression { func }
}

impl<FN, F> WrapSealed<F> for Decompression<FN>
where
    FN: Fn(CompressionProps) -> Response + Clone + Send,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Wrapped = WithDecompression<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithDecompression {
            filter,
            decompress: self.clone(),
        }
    }
}

mod internal {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use bytes::Bytes;
    use futures_util::{ready, Stream, TryFuture};
    use hyper::Body;
    use pin_project::pin_project;

    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::{Reply, Response};

    use super::Decompression;

    #[pin_project]
    #[derive(Debug)]
    pub struct DecompressableBody<S, E>
    where
        E: std::error::Error,
        S: Stream<Item = Result<Bytes, E>>,
    {
        #[pin]
        body: S,
    }

    impl<S, E> Stream for DecompressableBody<S, E>
    where
        E: std::error::Error,
        S: Stream<Item = Result<Bytes, E>>,
    {
        type Item = std::io::Result<Bytes>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            use std::io::{Error, ErrorKind};

            let pin = self.project();
            S::poll_next(pin.body, cx).map_err(|_| Error::from(ErrorKind::InvalidData))
        }
    }

    impl From<Body> for DecompressableBody<Body, hyper::Error> {
        fn from(body: Body) -> Self {
            DecompressableBody { body }
        }
    }

    #[derive(Debug)]
    pub struct DecompressionProps {
        pub(super) body: DecompressableBody<Body, hyper::Error>,
        pub(super) head: http::response::Parts,
    }

    impl From<http::Response<Body>> for DecompressionProps {
        fn from(resp: http::Response<Body>) -> Self {
            let (head, body) = resp.into_parts();
            DecompressionProps {
                body: body.into(),
                head,
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct Decompressed(pub(super) Response);

    impl Reply for Decompressed {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithDecompression<FN, F> {
        pub(super) decompress: Decompression<FN>,
        pub(super) filter: F,
    }

    impl<FN, F> FilterBase for WithDecompression<FN, F>
    where
        FN: Fn(DecompressionProps) -> Response + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Extract = (Decompressed,);
        type Error = F::Error;
        type Future = WithDecompressionFuture<FN, F::Future>;

        fn filter(&self, _: Internal) -> Self::Future {
            WithDecompressionFuture {
                decompress: self.decompress.clone(),
                future: self.filter.filter(Internal),
            }
        }
    }

    #[allow(missing_debug_implementations)]
    #[pin_project]
    pub struct WithDecompressionFuture<FN, F> {
        decompress: Decompression<FN>,
        #[pin]
        future: F,
    }

    impl<FN, F> Future for WithDecompressionFuture<FN, F>
    where
        FN: Fn(DecompressionProps) -> Response,
        F: TryFuture,
        F::Ok: Reply,
        F::Error: IsReject,
    {
        type Output = Result<Decompressed, F::Error>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let pin = self.as_mut().project();
            let result = ready!(pin.future.try_poll(cx));
            match result {
                Ok(reply) => {
                    let resp = (self.decompress.func)(reply.into_response().into());
                    Poll::Ready(Ok((Decompressed(resp),)))
                }
                Err(reject) => Poll::Ready(Err(reject)),
            }
        }
    }
}
