//! File System Filters

use std::cmp;
use std::io;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::{future, Future};
use futures::future::Either;
use http;
use hyper::{body::{Body, Sender}, rt};
use mime_guess;
use tokio::fs::File as TkFile;
use tokio::io::AsyncRead;
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
        .and(::path::tail())
        .and_then(move |tail: ::path::Tail| {
            let mut buf = PathBuf::from(base.as_ref());
            let p = match decode(tail.as_str()) {
                Ok(p) => p,
                Err(e) => {
                    debug!("dir: failed to decode route={:?}: {:?}", tail.as_str(), e);
                    return Either::A(future::err(reject::bad_request()));
                }
            };
            trace!("dir? base={:?}, route={:?}", base, p);
            for seg in p.split('/') {
                if seg.starts_with("..") {
                    debug!("dir: rejecting segment starting with '..'");
                    return Either::A(future::err(reject::bad_request()));
                } else {
                    buf.push(seg);
                }

            }

            trace!("dir: {:?}", buf);
            let path = Arc::new(buf);

            Either::B(file_reply(ArcPath(path.clone()))
                .map(|resp| File {
                    resp,
                }))
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
                        reject::not_found()
                    },
                    // There are actually other errors that could
                    // occur that really mean a 404, but the kind
                    // return is Other, making it hard to tell.
                    //
                    // A fix would be to check `Path::is_file` first,
                    // using `tokio_threadpool::blocking` around it...
                    _ => {
                        warn!("file open error: {} ", err);
                        reject::server_error()
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

        let (tx, body) = Body::channel();

        rt::spawn(copy_to_body(f.take().unwrap(), tx, buf_size, len));

        let content_type = mime_guess::guess_mime_type(path.as_ref());

        Ok(http::Response::builder()
            .status(200)
            .header("content-length", len)
            .header("content-type", content_type.as_ref())
            .body(body)
            .unwrap().into())
    })
        .map_err(|err: ::std::io::Error| {
            trace!("file metadata error: {}", err);
            reject::server_error()
        })
}

fn copy_to_body(mut f: TkFile, mut tx: Sender, buf_size: usize, mut len: u64) -> impl Future<Item=(), Error=()> + Send {
    let mut buf = BytesMut::new();
    future::poll_fn(move || loop {
        if len == 0 {
            return Ok(().into());
        }
        try_ready!(tx.poll_ready().map_err(|err| {
            trace!("body channel error while writing file: {}", err);
        }));
        if buf.remaining_mut() < buf_size {
            buf.reserve(buf_size);
        }
        let n = try_ready!(f.read_buf(&mut buf).map_err(|err| {
            trace!("file read error: {}", err);
        })) as u64;
        if n == 0 {
            return Ok(().into());
        }

        let mut chunk = buf.take().freeze();
        if n > len {
            chunk = chunk.split_to(len as usize);
            len = 0;
        } else {
            len -= n;
        }

        tx.send_data(chunk.into()).map_err(|_| {
            trace!("body channel error, rejected send_data");
        })?;
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
fn get_block_size(metadata: &Metadata) -> usize {
    8_192
}

