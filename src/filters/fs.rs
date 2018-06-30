//! dox?
use std::io;
use std::path::Path;
use std::sync::Arc;

use bytes::{BufMut, BytesMut};
use futures::{future, Future};
use futures::future::Either;
use http;
use tokio::fs;
use tokio::io::AsyncRead;

use ::filter::{Cons, HCons, FilterAnd, filter_fn};
use ::never::Never;
use ::reply::{Reply, Response, WarpBody};

/// Creates a `Filter` that serves a File at the `path`.
pub fn file(path: impl AsRef<Path> + Send + Sync + 'static) -> impl FilterAnd<Extract=Cons<File>> {
    let path = Arc::new(path);
    filter_fn(move || {
        trace!("file: {:?}", (*path).as_ref());
        Some(HCons(File {
            path: ArcPath(path.clone()),
        }, ()))
    })
}

/// dox?
pub struct File {
    path: ArcPath,
}

// Silly wrapper since Arc<AsRef<Path>> doesn't implement AsRef<Path> ;_;
struct ArcPath(Arc<AsRef<Path> + Send + Sync>);

impl AsRef<Path> for ArcPath {
    fn as_ref(&self) -> &Path {
        (*self.0).as_ref()
    }
}

impl Reply for File {
    type Future = Box<Future<Item=Response, Error=Never> + Send>;
    fn into_response(self) -> Self::Future {
        Box::new(file_reply(self.path))
    }
}

fn file_reply(path: ArcPath) -> impl Future<Item=Response, Error=Never> + Send {
    fs::File::open(path)
        .then(|res| match res {
            Ok(f) => Either::A(file_metadata(f)),
            Err(err) => {
                debug!("file open error: {}", err);
                let code = match err.kind() {
                    io::ErrorKind::NotFound => 404,
                    _ => 500,
                };

                let resp = Response(http::Response::builder()
                    .status(code)
                    .body(WarpBody::default())
                    .unwrap());
                Either::B(future::ok(resp))
            }
        })
}

fn file_metadata(f: fs::File) -> impl Future<Item=Response, Error=Never> + Send {
    let mut f = Some(f);
    future::poll_fn(move || {
        let meta = try_ready!(f.as_mut().unwrap().poll_metadata());
        let len = meta.len();

        let (tx, body) = ::hyper::Body::channel();

        ::hyper::rt::spawn(copy_to_body(f.take().unwrap(), tx, len));

        Ok(Response(http::Response::builder()
            .status(200)
            .header("content-length", len)
            .body(WarpBody::wrap(body))
            .unwrap()).into())
    })
        .or_else(|err: ::std::io::Error| {
            trace!("file metadata error: {}", err);

            Ok(Response(http::Response::builder()
                .status(500)
                .body(WarpBody::default())
                .unwrap()))
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
