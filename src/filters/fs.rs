//! dox?
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::{future, Future};
use futures::future::Either;
use http;
use tokio::fs;
use tokio::io::AsyncRead;

use ::error::Kind;
use ::filter::{cons, Cons, FilterClone, filter_fn};
use ::reply::{Reply, Response};

/// Creates a `Filter` that serves a File at the `path`.
pub fn file(path: impl Into<PathBuf>) -> impl FilterClone<Extract=Cons<File>, Error=::Error> {
    let path = Arc::new(path.into());
    filter_fn(move |_| {
        trace!("file: {:?}", path);

        file_reply(ArcPath(path.clone()))
            .map(|resp| cons(File {
                resp,
            }))
    })
}

/// Creates a `Filter` that serves a File at the `path`.
pub fn dir(path: impl Into<PathBuf>) -> impl FilterClone<Extract=Cons<File>, Error=::Error> {
    let base = Arc::new(path.into());
    filter_fn(move |route| {
        let mut buf = PathBuf::from(base.as_ref());

        //TODO: this could probably be factored out into a `path::tail()`
        //or similar Filter...

        let end = {
            let p = route.path();
            trace!("dir? base={:?}, route={:?}", base, p);
            for seg in p.split('/') {
                if seg.starts_with("..") {
                    debug!("dir: rejecting segment starting with '..'");
                    return Either::A(future::err(Kind::BadRequest.into()));
                } else {
                    buf.push(seg);
                }

            }
            p.len()
        };
        route.set_unmatched_path(end);


        trace!("dir: {:?}", buf);
        let path = Arc::new(buf);

        Either::B(file_reply(ArcPath(path.clone()))
            .map(|resp| cons(File {
                resp,
            })))
    })
}

/// dox?
pub struct File {
    resp: Response,
}

// Silly wrapper since Arc<AsRef<Path>> doesn't implement AsRef<Path> ;_;
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

fn file_reply(path: ArcPath) -> impl Future<Item=Response, Error=::Error> + Send {
    fs::File::open(path)
        .then(|res| match res {
            Ok(f) => Either::A(file_metadata(f)),
            Err(err) => {
                debug!("file open error: {} ", err);
                let code = match err.kind() {
                    io::ErrorKind::NotFound => Kind::NotFound,
                    // There are actually other errors that could
                    // occur that really mean a 404, but the kind
                    // return is Other, making it hard to tell.
                    //
                    // A fix would be to check `Path::is_file` first,
                    // using `tokio_threadpool::blocking` around it...
                    _ => Kind::ServerError,
                };
                Either::B(future::err(code.into()))
            }
        })
}

fn file_metadata(f: fs::File) -> impl Future<Item=Response, Error=::Error> + Send {
    let mut f = Some(f);
    future::poll_fn(move || {
        let meta = try_ready!(f.as_mut().unwrap().poll_metadata());
        let len = meta.len();

        let (tx, body) = ::hyper::Body::channel();

        ::hyper::rt::spawn(copy_to_body(f.take().unwrap(), tx, len));

        Ok(http::Response::builder()
            .status(200)
            .header("content-length", len)
            .body(body)
            .unwrap().into())
    })
        .map_err(|err: ::std::io::Error| {
            trace!("file metadata error: {}", err);
            Kind::ServerError.into()
        })
}

fn copy_to_body(mut f: fs::File, mut tx: ::hyper::body::Sender, mut len: u64) -> impl Future<Item=(), Error=()> + Send {
    let mut buf = BytesMut::new();
    future::poll_fn(move || loop {
        if len == 0 {
            return Ok(().into());
        }
        try_ready!(tx.poll_ready().map_err(|err| {
            trace!("body channel error while writing file: {}", err);
        }));
        if buf.remaining_mut() < 4096 {
            buf.reserve(4096 * 4);
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
