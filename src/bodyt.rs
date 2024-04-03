use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Buf;
use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::body::Frame;

#[derive(Debug)]
pub struct Body(BoxBody<Bytes, crate::Error>);

impl Default for Body {
    fn default() -> Self {
        Body::empty()
    }
}

impl hyper::body::Body for Body {
    type Data = Bytes;
    type Error = crate::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        self.0.size_hint()
    }
}

impl Body {
    pub(crate) fn empty() -> Self {
        Body(
            http_body_util::Empty::<Bytes>::new()
                .map_err(crate::Error::new)
                .boxed(),
        )
    }

    pub(crate) fn wrap<B>(body: B) -> Self
    where
        B: hyper::body::Body + Send + Sync + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let body = body
            .map_frame(|f| f.map_data(|mut buf| buf.copy_to_bytes(buf.remaining())))
            .map_err(crate::Error::new);
        Body(http_body_util::BodyExt::boxed(body))
    }

    pub(crate) fn wrap_stream<S, B, E>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = Result<B, E>> + Send + Sync + 'static,
        B: Into<Bytes>,
        E: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        let body = http_body_util::StreamBody::new(stream.map(|item| {
            item.map(|buf| Frame::data(buf.into()))
                .map_err(crate::Error::new)
        }));
        Body(http_body_util::BodyExt::boxed(body))
    }
}

impl From<Bytes> for Body {
    fn from(b: Bytes) -> Self {
        Body(
            http_body_util::Full::new(b)
                .map_err(crate::Error::new)
                .boxed(),
        )
    }
}

impl From<&'static str> for Body {
    fn from(s: &'static str) -> Self {
        Bytes::from(s).into()
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Bytes::from(s).into()
    }
}

impl From<Vec<u8>> for Body {
    fn from(v: Vec<u8>) -> Self {
        Bytes::from(v).into()
    }
}

impl From<Option<Bytes>> for Body {
    fn from(opt: Option<Bytes>) -> Self {
        match opt {
            Some(b) => b.into(),
            None => Body::empty(),
        }
    }
}
