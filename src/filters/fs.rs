//! File System Filters

use std::cmp;
use std::io;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::{future, Future, stream, Stream};
use futures::future::Either;
use http;
use hyper::{Body, Chunk};
use mime_guess;
use tokio::fs::File as TkFile;
use tokio::io::AsyncRead;
use tokio_threadpool;
use urlencoding::decode;

use ::filter::{Filter, FilterClone, filter_fn, One, one};
use ::reject::{self, Rejection};
use ::reply::{ReplySealed, Response};

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
pub fn file(path: impl Into<PathBuf>) -> impl FilterClone<Extract=One<File>, Error=Rejection> {
    let path = Arc::new(path.into());
    filter_fn(move |_| {
        trace!("file: {:?}", path);

        file_reply(ArcPath(path.clone()))
            .map(|resp| one(File {
                resp,
            }))
    })
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
pub fn dir(path: impl Into<PathBuf>) -> impl FilterClone<Extract=One<File>, Error=Rejection> {
    let base = Arc::new(path.into());
    ::get2()
        .and(path_from_tail(base))
        .and_then(|path| {
            file_reply(path)
                .map(|resp| File {
                    resp,
                })
        })
}

fn path_from_tail(base: Arc<PathBuf>) -> impl FilterClone<Extract=One<ArcPath>, Error=Rejection> {
    ::path::tail()
        .and_then(move |tail: ::path::Tail| {
            let mut buf = PathBuf::from(base.as_ref());
            let p = match decode(tail.as_str()) {
                Ok(p) => p,
                Err(err) => {
                    debug!("dir: failed to decode route={:?}: {:?}", tail.as_str(), err);
                    // FromUrlEncodingError doesn't implement StdError
                    return Err(reject::bad_request().with("dir: failed to decode route"));
                }
            };
            trace!("dir? base={:?}, route={:?}", base, p);
            for seg in p.split('/') {
                if seg.starts_with("..") {
                    debug!("dir: rejecting segment starting with '..'");
                    return Err(reject::bad_request().with("dir: rejecting segment"));
                } else {
                    buf.push(seg);
                }

            }

            Ok(buf)
        })
        .and_then(|buf: PathBuf| {
            // Checking Path::is_dir can block since it has to read from disk,
            // so put it in a blocking() future
            let mut buf = Some(buf);
            future::poll_fn(move || {
                let is_dir = try_ready!(tokio_threadpool::blocking(|| {
                    buf.as_ref().unwrap().is_dir()
                }));
                let mut buf = buf.take().unwrap();
                if is_dir {
                    debug!("dir: appending index.html to directory path");
                    buf.push("index.html");
                }

                trace!("dir: {:?}", buf);

                Ok(ArcPath(Arc::new(buf)).into())
            })
                .map_err(|blocking_err: tokio_threadpool::BlockingError| {
                    let err = format!(
                        "threadpool blocking error checking buf.is_dir(): {}",
                        blocking_err,
                    );
                    error!("{}", err);
                    reject::server_error().with(err)
                })
        })
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

impl ReplySealed for File {
    fn into_response(self) -> Response {
        self.resp
    }
}

fn file_reply(path: ArcPath) -> impl Future<Item=Response, Error=Rejection> + Send {
    TkFile::open(path.clone())
        .then(move |res| match res {
            Ok(f) => Either::A(file_metadata(f, path)),
            Err(err) => {
                let rej = match err.kind() {
                    io::ErrorKind::NotFound => {
                        debug!("file open error: {} ", err);
                        reject::not_found().with(err)
                    },
                    _ => {
                        warn!("file open error: {} ", err);
                        reject::server_error().with(err)
                    },
                };
                Either::B(future::err(rej))
            }
        })
}

fn file_metadata(f: TkFile, path: ArcPath) -> impl Future<Item=Response, Error=Rejection> + Send {
    let mut f = Some(f);
    future::poll_fn(move || {
        let meta = try_ready!(f.as_mut().unwrap().poll_metadata());
        let len = meta.len();
        let buf_size = optimal_buf_size(&meta);

        let stream = file_stream(f.take().unwrap(), buf_size, len);
        let body = Body::wrap_stream(stream);

        let content_type = mime_guess::guess_mime_type(path.as_ref());

        let mut res = http::Response::builder();
        res.status(200)
           .header("content-length", len)
           .header("content-type", content_type.as_ref());

        if let Some(modified) = last_modified(&meta) {
            res.header("last-modified", modified);
        }

        Ok(res
            .body(body)
            .unwrap().into())
    })
        .map_err(|err: ::std::io::Error| {
            debug!("file metadata error: {}", err);
            reject::server_error().with(err)
        })
}

fn last_modified(metadata: &Metadata) -> Option<String> {
    use httpdate::fmt_http_date;

    metadata
        .modified()
        .ok()
        .map(fmt_http_date)
}

fn file_stream(mut f: TkFile, buf_size: usize, mut len: u64) -> impl Stream<Item=Chunk, Error=io::Error> + Send {
    let mut buf = BytesMut::new();
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
