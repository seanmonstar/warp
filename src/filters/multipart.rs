//! Multipart body filters
//!
//! Filters that extract a multipart body for a route.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, io};

use bytes::{Buf, Bytes};
use futures::{future, Stream};
use headers::ContentType;
use hyper::Body;
use mime::Mime;
use multiparty::headers::Headers;
use multiparty::server::owned_futures03::{FormData as FormDataInner, Part as PartInner};

use crate::filter::{Filter, FilterBase, Internal};
use crate::reject::{self, Rejection};

// If not otherwise configured, default to 2MB.
const DEFAULT_FORM_DATA_MAX_LENGTH: u64 = 1024 * 1024 * 2;

/// A `Filter` to extract a `multipart/form-data` body from a request.
///
/// Create with the `warp::multipart::form()` function.
#[derive(Debug, Clone)]
pub struct FormOptions {
    max_length: u64,
}

/// A `Stream` of multipart/form-data `Part`s.
///
/// Extracted with a `warp::multipart::form` filter.
pub struct FormData {
    inner: FormDataInner<BodyIoError>,
}

/// A single "part" of a multipart/form-data body.
///
/// Yielded from the `FormData` stream.
pub struct Part {
    headers: Headers,
    part: PartInner<BodyIoError>,
}

/// Create a `Filter` to extract a `multipart/form-data` body from a request.
///
/// The extracted `FormData` type is a `Stream` of `Part`s, and each `Part`
/// in turn is a `Stream` of bytes.
pub fn form() -> FormOptions {
    FormOptions {
        max_length: DEFAULT_FORM_DATA_MAX_LENGTH,
    }
}

// ===== impl Form =====

impl FormOptions {
    /// Set the maximum byte length allowed for this body.
    ///
    /// Defaults to 2MB.
    pub fn max_length(mut self, max: u64) -> Self {
        self.max_length = max;
        self
    }
}

type FormFut = Pin<Box<dyn Future<Output = Result<(FormData,), Rejection>> + Send>>;

impl FilterBase for FormOptions {
    type Extract = (FormData,);
    type Error = Rejection;
    type Future = FormFut;

    fn filter(&self, _: Internal) -> Self::Future {
        let boundary = super::header::header2::<ContentType>().and_then(|ct| {
            let mime = Mime::from(ct);
            let mime = mime
                .get_param("boundary")
                .map(|v| v.to_string())
                .ok_or_else(|| reject::invalid_header("content-type"));
            future::ready(mime)
        });

        let filt = super::body::content_length_limit(self.max_length)
            .and(boundary)
            .and(super::body::body())
            .map(|boundary: String, body| {
                let body = BodyIoError(body);
                FormData {
                    inner: FormDataInner::new(body, &boundary),
                }
            });

        let fut = filt.filter(Internal);

        Box::pin(fut)
    }
}

// ===== impl FormData =====

impl fmt::Debug for FormData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FormData").finish()
    }
}

impl Stream for FormData {
    type Item = Result<Part, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(part))) => {
                let headers = match part.raw_headers().parse() {
                    Ok(headers) => headers,
                    Err(err) => return Poll::Ready(Some(Err(crate::Error::new(err)))),
                };
                Poll::Ready(Some(Ok(Part { part, headers })))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(crate::Error::new(err)))),
        }
    }
}

// ===== impl Part =====

impl Part {
    /// Get the name of this part.
    pub fn name(&self) -> &str {
        &self.headers.name
    }

    /// Get the filename of this part, if present.
    pub fn filename(&self) -> Option<&str> {
        self.headers.filename.as_deref()
    }

    /// Get the content-type of this part, if present.
    pub fn content_type(&self) -> Option<&str> {
        self.headers.content_type.as_deref()
    }

    /// Asynchronously get some of the data for this `Part`.
    pub async fn data(&mut self) -> Option<Result<impl Buf, crate::Error>> {
        future::poll_fn(|cx| self.poll_next(cx)).await
    }

    /// Convert this `Part` into a `Stream` of `Buf`s.
    pub fn stream(self) -> impl Stream<Item = Result<impl Buf, crate::Error>> {
        PartStream(self)
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, crate::Error>>> {
        match Pin::new(&mut self.part).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(crate::Error::new(err)))),
        }
    }
}

impl fmt::Debug for Part {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("Part");
        builder.field("name", &self.headers.name);

        if let Some(ref filename) = self.headers.filename {
            builder.field("filename", filename);
        }

        if let Some(ref mime) = self.headers.content_type {
            builder.field("content_type", mime);
        }

        builder.finish()
    }
}

struct PartStream(Part);

impl Stream for PartStream {
    type Item = Result<Bytes, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.poll_next(cx)
    }
}

struct BodyIoError(Body);

impl Stream for BodyIoError {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => {
                Poll::Ready(Some(Err(io::Error::new(io::ErrorKind::Other, err))))
            }
        }
    }
}
