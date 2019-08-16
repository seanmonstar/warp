//! File System Filters

use std::cmp;
use std::error::Error as StdError;
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::future::Either;
use futures::{future, stream, Future, Stream};
use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMapExt, IfModifiedSince, IfRange,
    IfUnmodifiedSince, LastModified, Range,
};
use http::StatusCode;
use hyper::{Body, Chunk};
use mime_guess;
use tokio::fs::File as TkFile;
use tokio::io::AsyncRead;
use tokio_threadpool;
use urlencoding::decode;

use filter::{Filter, FilterClone, One};
use never::Never;
use reject::{self, Rejection};
use reply::{Reply, Response};

/// Creates a `Filter` that serves a File at the `path`.
///
/// Does not filter out based on any information of the request. Always serves
/// the file at the exact `path` provided. Thus, this can be used to serve a
/// single file with `GET`s, but could also be used in combination with other
/// filters, such as after validating in `POST` request, wanting to return a
/// specific file as the body.
///
/// For serving a directory, see [dir](dir).
///
/// # Example
///
/// ```
/// // Always serves this file from the file system.
/// let route = warp::fs::file("/www/static/app.js");
/// ```
///
/// # Note
///
/// This filter uses `tokio-fs` to serve files, which requires the server
/// to be run in the threadpool runtime. This is only important to remember
/// if starting a runtime manually.
pub fn file(path: impl Into<PathBuf>) -> impl FilterClone<Extract = One<File>, Error = Rejection> {
    let path = Arc::new(path.into());
    ::any()
        .map(move || {
            trace!("file: {:?}", path);
            ArcPath(path.clone())
        })
        .and(conditionals())
        .and_then(file_reply)
}

/// Creates a `Filter` that serves a directory at the base `path` joined
/// by the request path.
///
/// This can be used to serve "static files" from a directory. By far the most
/// common pattern of serving static files is for `GET` requests, so this
/// filter automatically includes a `GET` check.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Matches requests that start with `/static`,
/// // and then uses the rest of that path to lookup
/// // and serve a file from `/www/static`.
/// let route = warp::path("static")
///     .and(warp::fs::dir("/www/static"));
///
/// // For example:
/// // - `GET /static/app.js` would serve the file `/www/static/app.js`
/// // - `GET /static/css/app.css` would serve the file `/www/static/css/app.css`
/// ```
///
/// # Note
///
/// This filter uses `tokio-fs` to serve files, which requires the server
/// to be run in the threadpool runtime. This is only important to remember
/// if starting a runtime manually.
pub fn dir(path: impl Into<PathBuf>) -> impl FilterClone<Extract = One<File>, Error = Rejection> {
    let base = Arc::new(path.into());
    ::get2()
        .and(path_from_tail(base))
        .and(conditionals())
        .and_then(file_reply)
}

fn path_from_tail(
    base: Arc<PathBuf>,
) -> impl FilterClone<Extract = One<ArcPath>, Error = Rejection> {
    ::path::tail()
        .and_then(move |tail: ::path::Tail| {
            sanitize_path(base.as_ref(), tail.as_str())
        })
        .and_then(|buf: PathBuf| {
            // Checking Path::is_dir can block since it has to read from disk,
            // so put it in a blocking() future
            let mut buf = Some(buf);
            future::poll_fn(move || {
                let is_dir = try_ready!(tokio_threadpool::blocking(|| buf
                    .as_ref()
                    .unwrap()
                    .is_dir()));
                let mut buf = buf.take().unwrap();
                if is_dir {
                    debug!("dir: appending index.html to directory path");
                    buf.push("index.html");
                }

                trace!("dir: {:?}", buf);

                Ok(ArcPath(Arc::new(buf)).into())
            })
            .map_err(|blocking_err: tokio_threadpool::BlockingError| {
                error!(
                    "threadpool blocking error checking buf.is_dir(): {}",
                    blocking_err,
                );
                reject::known(FsNeedsTokioThreadpool)
            })
        })
}

fn sanitize_path(base: impl AsRef<Path>, tail: &str) -> Result<PathBuf, Rejection> {
    let mut buf = PathBuf::from(base.as_ref());
    let p = match decode(tail) {
        Ok(p) => p,
        Err(err) => {
            debug!("dir: failed to decode route={:?}: {:?}", tail, err);
            // FromUrlEncodingError doesn't implement StdError
            return Err(reject::not_found());
        }
    };
    trace!("dir? base={:?}, route={:?}", base.as_ref(), p);
    for seg in p.split('/') {
        if seg.starts_with("..") {
            warn!("dir: rejecting segment starting with '..'");
            return Err(reject::not_found());
        } else if seg.contains('\\') {
            warn!("dir: rejecting segment containing with backslash (\\)");
            return Err(reject::not_found());
        } else {
            buf.push(seg);
        }
    }
    Ok(buf)
}

#[derive(Debug)]
struct Conditionals {
    if_modified_since: Option<IfModifiedSince>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_range: Option<IfRange>,
    range: Option<Range>,
}

enum Cond {
    NoBody(Response),
    WithBody(Option<Range>),
}

impl Conditionals {
    fn check(self, last_modified: Option<LastModified>) -> Cond {
        if let Some(since) = self.if_unmodified_since {
            let precondition = last_modified
                .map(|time| since.precondition_passes(time.into()))
                .unwrap_or(false);

            trace!(
                "if-unmodified-since? {:?} vs {:?} = {}",
                since,
                last_modified,
                precondition
            );
            if !precondition {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::PRECONDITION_FAILED;
                return Cond::NoBody(res);
            }
        }

        if let Some(since) = self.if_modified_since {
            trace!(
                "if-modified-since? header = {:?}, file = {:?}",
                since,
                last_modified
            );
            let unmodified = last_modified
                .map(|time| !since.is_modified(time.into()))
                // no last_modified means its always modified
                .unwrap_or(false);
            if unmodified {
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::NOT_MODIFIED;
                return Cond::NoBody(res);
            }
        }

        if let Some(if_range) = self.if_range {
            trace!("if-range? {:?} vs {:?}", if_range, last_modified);
            let can_range = !if_range.is_modified(None, last_modified.as_ref());

            if !can_range {
                return Cond::WithBody(None);
            }
        }

        Cond::WithBody(self.range)
    }
}

fn conditionals() -> impl Filter<Extract = One<Conditionals>, Error = Never> + Copy {
    ::header::optional2()
        .and(::header::optional2())
        .and(::header::optional2())
        .and(::header::optional2())
        .map(
            |if_modified_since, if_unmodified_since, if_range, range| Conditionals {
                if_modified_since,
                if_unmodified_since,
                if_range,
                range,
            },
        )
}

/// A file response.
#[derive(Debug)]
pub struct File {
    resp: Response,
}

// Silly wrapper since Arc<PathBuf> doesn't implement AsRef<Path> ;_;
#[derive(Clone, Debug)]
struct ArcPath(Arc<PathBuf>);

impl AsRef<Path> for ArcPath {
    fn as_ref(&self) -> &Path {
        (*self.0).as_ref()
    }
}

impl Reply for File {
    fn into_response(self) -> Response {
        self.resp
    }
}

fn file_reply(
    path: ArcPath,
    conditionals: Conditionals,
) -> impl Future<Item = File, Error = Rejection> + Send {
    TkFile::open(path.clone()).then(move |res| match res {
        Ok(f) => Either::A(file_conditional(f, path, conditionals)),
        Err(err) => {
            let rej = match err.kind() {
                io::ErrorKind::NotFound => {
                    debug!("file not found: {:?}", path.as_ref().display());
                    reject::not_found()
                }
                _ => {
                    error!(
                        "file open error (path={:?}): {} ",
                        path.as_ref().display(),
                        err
                    );
                    reject::not_found()
                }
            };
            Either::B(future::err(rej))
        }
    })
}

fn file_metadata(f: TkFile) -> impl Future<Item = (TkFile, Metadata), Error = Rejection> {
    let mut f = Some(f);
    future::poll_fn(move || {
        let meta = try_ready!(f.as_mut().unwrap().poll_metadata());
        Ok((f.take().unwrap(), meta).into())
    })
    .map_err(|err: ::std::io::Error| {
        debug!("file metadata error: {}", err);
        reject::not_found()
    })
}

fn file_conditional(
    f: TkFile,
    path: ArcPath,
    conditionals: Conditionals,
) -> impl Future<Item = File, Error = Rejection> + Send {
    file_metadata(f).map(move |(file, meta)| {
        let mut len = meta.len();
        let modified = meta.modified().ok().map(LastModified::from);

        let resp = match conditionals.check(modified) {
            Cond::NoBody(resp) => resp,
            Cond::WithBody(range) => {
                bytes_range(range, len)
                    .map(|(start, end)| {
                        let sub_len = end - start;
                        let buf_size = optimal_buf_size(&meta);
                        let stream = file_stream(file, buf_size, (start, end));
                        let body = Body::wrap_stream(stream);

                        let mut resp = Response::new(body);

                        if sub_len != len {
                            *resp.status_mut() = StatusCode::PARTIAL_CONTENT;
                            resp.headers_mut().typed_insert(
                                ContentRange::bytes(start..end, len).expect("valid ContentRange"),
                            );

                            len = sub_len;
                        }

                        let mime = mime_guess::from_path(path.as_ref()).first_or_octet_stream();

                        resp.headers_mut().typed_insert(ContentLength(len));
                        resp.headers_mut().typed_insert(ContentType::from(mime));
                        resp.headers_mut().typed_insert(AcceptRanges::bytes());

                        if let Some(last_modified) = modified {
                            resp.headers_mut().typed_insert(last_modified);
                        }

                        resp
                    })
                    .unwrap_or_else(|BadRange| {
                        // bad byte range
                        let mut resp = Response::new(Body::empty());
                        *resp.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
                        resp.headers_mut()
                            .typed_insert(ContentRange::unsatisfied_bytes(len));
                        resp
                    })
            }
        };

        File { resp }
    })
}

struct BadRange;

fn bytes_range(range: Option<Range>, max_len: u64) -> Result<(u64, u64), BadRange> {
    use std::ops::Bound;

    let range = if let Some(range) = range {
        range
    } else {
        return Ok((0, max_len));
    };

    let ret = range
        .iter()
        .map(|(start, end)| {
            let start = match start {
                Bound::Unbounded => 0,
                Bound::Included(s) => s,
                Bound::Excluded(s) => s + 1,
            };

            let end = match end {
                Bound::Unbounded => max_len,
                Bound::Included(s) => s + 1,
                Bound::Excluded(s) => s,
            };

            if start < end && end <= max_len {
                Ok((start, end))
            } else {
                trace!("unsatisfiable byte range: {}-{}/{}", start, end, max_len);
                Err(BadRange)
            }
        })
        .next()
        .unwrap_or(Ok((0, max_len)));
    ret
}

fn file_stream(
    file: TkFile,
    buf_size: usize,
    (start, end): (u64, u64),
) -> impl Stream<Item = Chunk, Error = io::Error> + Send {
    use std::io::SeekFrom;

    // seek
    let seek = if start != 0 {
        trace!("partial content; seeking ({}..{})", start, end);
        Either::A(file.seek(SeekFrom::Start(start)).map(|(f, _pos)| f))
    } else {
        Either::B(future::ok(file))
    };

    seek.into_stream()
        .map(move |mut f| {
            let mut buf = BytesMut::new();
            let mut len = end - start;
            stream::poll_fn(move || {
                if len == 0 {
                    return Ok(None.into());
                }
                if buf.remaining_mut() < buf_size {
                    buf.reserve(buf_size);
                }
                let n = try_ready!(f.read_buf(&mut buf).map_err(|err| {
                    debug!("file read error: {}", err);
                    err
                })) as u64;

                if n == 0 {
                    debug!("file read found EOF before expected length");
                    return Ok(None.into());
                }

                let mut chunk = buf.take().freeze();
                if n > len {
                    chunk = chunk.split_to(len as usize);
                    len = 0;
                } else {
                    len -= n;
                }

                Ok(Some(Chunk::from(chunk)).into())
            })
        })
        .flatten()
}

fn optimal_buf_size(metadata: &Metadata) -> usize {
    let block_size = get_block_size(metadata);

    // If file length is smaller than block size, don't waste space
    // reserving a bigger-than-needed buffer.
    cmp::min(block_size as u64, metadata.len()) as usize
}

#[cfg(unix)]
fn get_block_size(metadata: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    //TODO: blksize() returns u64, should handle bad cast...
    //(really, a block size bigger than 4gb?)
    metadata.blksize() as usize
}

#[cfg(not(unix))]
fn get_block_size(_metadata: &Metadata) -> usize {
    8_192
}

// ===== Rejections =====

#[derive(Debug)]
pub(crate) struct FsNeedsTokioThreadpool;

impl ::std::fmt::Display for FsNeedsTokioThreadpool {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.write_str("File system operations require tokio threadpool runtime")
    }
}

impl StdError for FsNeedsTokioThreadpool {
    fn description(&self) -> &str {
        "File system operations require tokio threadpool runtime"
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_path;

    #[test]
    fn test_sanitize_path() {
        let base = "/var/www";

        fn p(s: &str) -> &::std::path::Path {
            s.as_ref()
        }

        assert_eq!(sanitize_path(base, "/foo.html").unwrap(), p("/var/www/foo.html"));

        // bad paths
        sanitize_path(base, "/../foo.html").expect_err("dot dot");

        sanitize_path(base, "/C:\\/foo.html").expect_err("C:\\");
    }
}
