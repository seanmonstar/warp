//! Multipart body filters
//!
//! Filters that extract a multipart body for a route.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fmt, mem};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{future, Stream};
use headers::ContentType;
use hyper::Body;
use mime::Mime;
use tokio::sync::Mutex;

use crate::filter::{Filter, FilterBase, Internal};
use crate::reject::{self, Rejection};
use crate::Error;

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
    boundary: Boundary,
    body: Arc<Mutex<Option<MultipartBody>>>,
    state: FormDataState,
}

enum FormDataState {
    Boundary,
    BoundarySuffix,
    Headers,
}

/// A single "part" of a multipart/form-data body.
///
/// Yielded from the `FormData` stream.
pub struct Part {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,

    boundary: Boundary,
    body: Option<Arc<Mutex<Option<MultipartBody>>>>,
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
            .map(|boundary: String, body| FormData {
                boundary: Boundary::new(&boundary),
                body: Arc::new(Mutex::new(Some(MultipartBody::new(body)))),

                state: FormDataState::Boundary,
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
        let Self {
            boundary,
            body: self_body,

            state,
        } = &mut *self;
        let option_body = match Arc::get_mut(self_body) {
            Some(body) => {
                // We have exclusive access to body
                Mutex::get_mut(body)
            }
            None => {
                // An old `Part` has been kept around
                let body = match self_body.try_lock() {
                    Ok(mut body) => mem::take(&mut *body),
                    Err(_) => {
                        // Something is holding the lock, but it should release it soon
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                };

                // We took body out of the other `Part`'s `Arc`, leaving a `None` in its place,
                // now make a new `Arc`
                *self_body = Arc::new(Mutex::new(body));

                Mutex::get_mut(
                    Arc::get_mut(self_body).expect("self.body is a new Arc, so it must be unique"),
                )
            }
        };

        let body = match option_body {
            Some(body) => body,
            None => {
                // body is `None`, which means that we reached the end of the multipart stream
                return Poll::Ready(None);
            }
        };

        loop {
            match *state {
                FormDataState::Boundary => {
                    // skip the boundary

                    match body.poll_next_until_after_boundary(cx, boundary.with_dashes()) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Some(Ok((bytes, false)))) => drop(bytes),
                        Poll::Ready(Some(Ok((bytes, true)))) => {
                            drop(bytes);

                            *state = FormDataState::BoundarySuffix;
                        }
                        Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                        Poll::Ready(None) => return Poll::Ready(None),
                    }
                }
                FormDataState::BoundarySuffix => {
                    // Read the 2 bytes after the boundary to determine if there's another `Part` after it

                    let mut bytes1 = match body.poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Some(Ok(bytes))) => bytes,
                        Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                        Poll::Ready(None) => return Poll::Ready(None),
                    };

                    if bytes1.len() < 2 {
                        // Unlikely: the chunk is less than two bytes

                        let bytes2 = match body.poll_next(cx) {
                            Poll::Pending => {
                                body.rewind(bytes1);
                                return Poll::Pending;
                            }
                            Poll::Ready(Some(Ok(bytes))) => bytes,
                            Poll::Ready(Some(Err(err))) => {
                                body.rewind(bytes1);
                                return Poll::Ready(Some(Err(err)));
                            }
                            Poll::Ready(None) => {
                                body.rewind(bytes1);
                                return Poll::Ready(None);
                            }
                        };

                        bytes1 = join_bytes(bytes1, bytes2);

                        if bytes1.len() < 2 {
                            // Even more unlikely: the two chunks combined are less than 2 bytes
                            body.rewind(bytes1);
                            continue;
                        }
                    }

                    if bytes1.starts_with(b"\r\n") {
                        // There's another part after this one

                        bytes1.advance(2);
                        body.rewind(bytes1);

                        *state = FormDataState::Headers;
                    } else if bytes1.starts_with(b"--") {
                        // There are no more parts

                        drop(body);
                        *option_body = None;
                        return Poll::Ready(None);
                    } else {
                        return Poll::Ready(Some(Err(Error::new("Unexpected suffix"))));
                    }
                }
                FormDataState::Headers => {
                    // Read the headers

                    let mut bytes = match body.poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Some(Ok(bytes))) => bytes,
                        Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                        Poll::Ready(None) => return Poll::Ready(None),
                    };

                    // Read headers

                    let mut headers = [httparse::EMPTY_HEADER; 8];
                    let (read, headers) = match httparse::parse_headers(&bytes, &mut headers) {
                        Ok(httparse::Status::Complete((read, headers))) => (read, headers),
                        Ok(httparse::Status::Partial) => match body.poll_next(cx) {
                            Poll::Pending => {
                                body.rewind(bytes);
                                return Poll::Pending;
                            }
                            Poll::Ready(Some(Ok(bytes2))) => {
                                body.rewind(join_bytes(bytes, bytes2));

                                continue;
                            }
                            Poll::Ready(Some(Err(err))) => {
                                body.rewind(bytes);
                                return Poll::Ready(Some(Err(err)));
                            }
                            Poll::Ready(None) => {
                                body.rewind(bytes);
                                return Poll::Ready(None);
                            }
                        },
                        Err(err) => return Poll::Ready(Some(Err(crate::Error::new(err)))),
                    };

                    // Find the content-disposition header, error if it is not found
                    let content_disposition = match headers
                        .iter()
                        .find(|header| header.name.eq_ignore_ascii_case("content-disposition"))
                    {
                        Some(header) => match std::str::from_utf8(&header.value) {
                            Ok(content_dispositon) => content_dispositon,
                            Err(err) => return Poll::Ready(Some(Err(crate::Error::new(err)))),
                        },
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                "No Content-Disposition header found for this multipart part",
                            ))));
                        }
                    };

                    // Find the content-type header
                    let content_type = match headers
                        .iter()
                        .find(|header| header.name.eq_ignore_ascii_case("content-type"))
                    {
                        Some(header) => match std::str::from_utf8(&header.value) {
                            Ok(content_type) => Some(content_type.to_string()),
                            Err(err) => return Poll::Ready(Some(Err(crate::Error::new(err)))),
                        },
                        None => None,
                    };

                    let content_disposition = match content_disposition.strip_prefix("form-data") {
                        Some(content_disposition) => content_disposition,
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                "Content-Disposition doesn't begin with 'form-data'",
                            ))))
                        }
                    };

                    // Parse the `name` and `filename` from the content-disposition
                    let mut name = None;
                    let mut filename = None;

                    for param in content_disposition.split(';').skip(1) {
                        let param = param.trim();

                        let mut splitter = param.split('=');
                        let param_name = splitter.next().expect("always Some");

                        if param_name != "name" && param_name != "filename" {
                            continue;
                        }

                        let param_value = match splitter.next() {
                            Some(value) => value,
                            None => {
                                return Poll::Ready(Some(Err(Error::new(
                                    "Invalid Content-Disposition parameter value",
                                ))))
                            }
                        };
                        let param_value =
                            param_value.trim_matches(|c: char| c.is_whitespace() || c == '"');

                        if param_name == "name" {
                            name = Some(param_value);
                        } else {
                            filename = Some(param_value);
                        }
                    }

                    let name = match name {
                        Some(name) => name.to_string(),
                        None => {
                            return Poll::Ready(Some(Err(Error::new(
                                "Content-Disposition 'name' parameter not found",
                            ))));
                        }
                    };
                    let filename = filename.map(|filename| filename.to_string());

                    // Skip the header bytes
                    bytes.advance(read);
                    body.rewind(bytes);

                    // Prepare for the next `poll_next`
                    *state = FormDataState::Boundary;

                    return Poll::Ready(Some(Ok(Part {
                        name,
                        filename,
                        content_type,

                        boundary: self.boundary.clone(),
                        body: Some(Arc::clone(&self.body)),
                    })));
                }
            }
        }
    }
}

// ===== impl Part =====

impl Part {
    /// Get the name of this part.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the filename of this part, if present.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_ref().map(|s| &**s)
    }

    /// Get the content-type of this part, if present.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_ref().map(|s| &**s)
    }

    /// Asynchronously get some of the data for this `Part`.
    pub async fn data(&mut self) -> Option<Result<impl Buf, crate::Error>> {
        future::poll_fn(|cx| self.poll_data(cx)).await
    }

    fn poll_data(&mut self, cx: &mut Context) -> Poll<Option<Result<Bytes, crate::Error>>> {
        let arc_body = match &self.body {
            Some(arc_body) => arc_body,
            None => {
                // If `self.body` is `None`, this `Part` has been exhausted
                return Poll::Ready(None);
            }
        };

        let mut guard_body = match arc_body.try_lock() {
            Ok(guard_body) => guard_body,
            Err(_) => {
                // If something else is playing with the lock this `Part` isn't the last one
                return Poll::Ready(Some(Err(Error::new(
                    "Tried to poll data from the not last Part",
                ))));
            }
        };

        let body = match &mut *guard_body {
            Some(body) => body,
            None => {
                // If `body` is None this `Part` isn't the last one
                return Poll::Ready(Some(Err(Error::new(
                    "Tried to poll data from the not last Part",
                ))));
            }
        };

        match body.poll_next_until_boundary(cx, self.boundary.with_new_line_and_dashes()) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok((bytes, false)))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Some(Ok((bytes, true)))) => {
                drop(guard_body);
                self.body = None;

                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }

    /// Convert this `Part` into a `Stream` of `Buf`s.
    pub fn stream(self) -> impl Stream<Item = Result<impl Buf, crate::Error>> {
        PartStream(self)
    }
}

impl fmt::Debug for Part {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("Part");
        builder.field("name", &self.name);

        if let Some(ref filename) = self.filename {
            builder.field("filename", filename);
        }

        if let Some(ref mime) = self.content_type {
            builder.field("content_type", mime);
        }

        builder.finish()
    }
}

struct PartStream(Part);

impl Stream for PartStream {
    type Item = Result<Bytes, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.poll_data(cx)
    }
}

// ===== impl Boundary =====

/// A multipart boundary stored as `\r\n--{boundary}`
#[derive(Debug, Clone)]
struct Boundary(Bytes);

impl Boundary {
    fn new(boundary: &str) -> Self {
        Self(format!("\r\n--{}", boundary).into())
    }

    /// Equivalent to `format!("--{}", boundary)`
    fn with_dashes(&self) -> &[u8] {
        &self.0["\r\n".len()..]
    }

    /// Equivalent to `format!("\r\n--{}", boundary)`
    fn with_new_line_and_dashes(&self) -> &[u8] {
        &self.0
    }
}

// ===== impl Multipart Body =====

struct MultipartBody {
    body: Body,

    buf: Bytes,
}

impl MultipartBody {
    fn new(body: Body) -> Self {
        Self {
            body,

            buf: Bytes::new(),
        }
    }

    fn rewind(&mut self, buf: Bytes) {
        debug_assert!(self.buf.is_empty());

        self.buf = buf;
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, crate::Error>>> {
        if !self.buf.is_empty() {
            Poll::Ready(Some(Ok(mem::take(&mut self.buf))))
        } else {
            match Pin::new(&mut self.body).poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(bytes))),
                Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(crate::Error::new(err)))),
                Poll::Ready(None) => Poll::Ready(None),
            }
        }
    }

    fn poll_next_until_after_boundary(
        &mut self,
        cx: &mut Context<'_>,
        boundary: &[u8],
    ) -> Poll<Option<Result<(Bytes, bool), crate::Error>>> {
        match self.poll_next_until_boundary(cx, boundary) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok((bytes, false)))) => Poll::Ready(Some(Ok((bytes, false)))),
            Poll::Ready(Some(Ok((bytes, true)))) => {
                self.buf.advance(boundary.len());
                Poll::Ready(Some(Ok((bytes, true))))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }

    fn poll_next_until_boundary(
        &mut self,
        cx: &mut Context<'_>,
        boundary: &[u8],
    ) -> Poll<Option<Result<(Bytes, bool), crate::Error>>> {
        let mut bytes1 = match self.poll_next(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Some(Ok(bytes))) => bytes,
            Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => return Poll::Ready(None),
        };

        if bytes1.len() >= boundary.len() {
            match find_bytes(&bytes1, boundary) {
                Some(i) => {
                    // Boundary has been found. Return the section of `bytes1` before the start
                    // of the boundary, and save the rest of it for the next call.
                    self.rewind(bytes1.split_off(i));
                    Poll::Ready(Some(Ok((bytes1, true))))
                }
                None => {
                    // No boundary has been found. Return `bytes1`, except the last `boundary.len()`
                    // bytes, which are instead saved for next time.
                    self.rewind(bytes1.split_off(bytes1.len() - boundary.len() + 1));
                    Poll::Ready(Some(Ok((bytes1, false))))
                }
            }
        } else {
            let bytes2 = match self.poll_next(cx) {
                Poll::Pending => {
                    self.rewind(bytes1);
                    return Poll::Pending;
                }
                Poll::Ready(Some(Ok(bytes))) => bytes,
                Poll::Ready(Some(Err(err))) => {
                    self.rewind(bytes1);
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(None) => {
                    self.rewind(bytes1);
                    return Poll::Ready(None);
                }
            };

            if bytes1.len() + bytes2.len() < boundary.len() {
                // Unlikely: two reads yielded less than `boundary.len()`
                self.rewind(join_bytes(bytes1, bytes2));

                // Since it's unlikely return `Pending` and immediately wake the runtime
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                match find_bytes_split(&bytes1, &bytes2, boundary) {
                    Some(i) => {
                        // Boundary sits between `bytes1` and `bytes2`, starting inside `bytes1`.
                        // Return everything before it and save everything after it for the next call.
                        self.rewind(join_bytes(bytes1.split_off(i), bytes2));
                        Poll::Ready(Some(Ok((bytes1, true))))
                    }
                    None => {
                        // No boundary has been found, yield `bytes1 + bytes2` except for the last `boundary.len() - 1` bytes
                        let mut bytes = join_bytes(bytes1, bytes2);

                        self.rewind(bytes.split_off(bytes.len() - boundary.len() + 1));
                        Poll::Ready(Some(Ok((bytes, false))))
                    }
                }
            }
        }
    }
}

fn find_bytes(bytes: &[u8], pattern: &[u8]) -> Option<usize> {
    twoway::find_bytes(bytes, pattern)
}

fn find_bytes_split(mut bytes1: &[u8], bytes2: &[u8], pattern: &[u8]) -> Option<usize> {
    let mut i = 0;

    while !bytes1.is_empty() && bytes1.len() + bytes2.len() >= pattern.len() {
        let skip1 = bytes1.len().min(pattern.len());

        let (pattern1, pattern2) = pattern.split_at(skip1);
        if &bytes1[..skip1] == pattern1 && bytes2.starts_with(pattern2) {
            return Some(i);
        }

        bytes1 = &bytes1[1..];
        i += 1;
    }

    None
}

fn join_bytes(bytes1: Bytes, bytes2: Bytes) -> Bytes {
    if bytes1.is_empty() {
        bytes2
    } else if bytes2.is_empty() {
        bytes1
    } else {
        let mut buf = BytesMut::with_capacity(bytes1.len() + bytes2.len());
        buf.put(bytes1);
        buf.put(bytes2);
        buf.freeze()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_bytes() {
        assert_eq!(find_bytes(b"abcdefgh", b"abc"), Some(0));
        assert_eq!(find_bytes(b"abc", b"abc"), Some(0));
        assert_eq!(find_bytes(b"abcdefgh", b"bcde"), Some(1));
        assert_eq!(find_bytes(b"abcdefgh", b"bc"), Some(1));
    }

    #[test]
    fn search_bytes_split() {
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"abc"), Some(0));
        assert_eq!(find_bytes_split(b"abc", b"", b"abc"), Some(0));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"bcde"), Some(1));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"bc"), Some(1));
        assert_eq!(find_bytes_split(b"abcd", b"efgh", b"fh"), None);
    }
}
