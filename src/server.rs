use std::net::SocketAddr;
use std::sync::Arc;

use futures::{Async, Future, Poll, Stream};
use hyper::{rt, Server as HyperServer};
use hyper::service::{service_fn};
use tokio_io::{AsyncRead, AsyncWrite};

use ::never::Never;
use ::reject::Reject;
use ::reply::{ReplySealed, Reply};
use ::Request;

/// Create a `Server` with the provided service.
pub fn serve<S>(service: S) -> Server<S>
where
    S: IntoWarpService + 'static,
{
    Server {
        pipeline: false,
        service,
    }
}

/// A Warp Server ready to filter requests.
#[derive(Debug)]
pub struct Server<S> {
    pipeline: bool,
    service: S,
}

// Getting all various generic bounds to make this a re-usable method is
// very complicated, so instead this is just a macro.
macro_rules! into_service {
    ($this:ident) => ({
        let inner = Arc::new($this.service.into_warp_service());
        move || {
            let inner = inner.clone();
            service_fn(move |req| {
                ReplyFuture {
                    inner: inner.call(req)
                }
            })
        }
    });
}
macro_rules! bind_inner {
    ($this:ident, $addr:expr) => ({
        let service = into_service!($this);
        let srv = HyperServer::bind(&$addr.into())
            .http1_pipeline_flush($this.pipeline)
            .serve(service);
        let addr = srv.local_addr();
        (addr, srv)
    });
}

impl<S> Server<S>
where
    S: IntoWarpService + 'static,
    <<S::Service as WarpService>::Reply as Future>::Item: Reply + Send,
    <<S::Service as WarpService>::Reply as Future>::Error: Reject + Send,
{
    /// Run this `Server` forever on the current thread.
    pub fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        info!("warp drive engaged: listening on {}", addr);

        rt::run(fut);
    }

    /// Run this `Server` forever on the current thread with a specific stream
    /// of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    pub fn run_incoming<I>(self, incoming: I)
    where
        I: Stream + Send + 'static,
        I::Item: AsyncRead + AsyncWrite + Send + 'static,
        I::Error: Into<Box<::std::error::Error + Send + Sync>>,
    {
        let fut = self.serve_incoming(incoming);

        info!("warp drive engaged: listening with custom incoming");

        rt::run(fut);
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    pub fn bind(self, addr: impl Into<SocketAddr> + 'static) -> impl Future<Item=(), Error=()> + 'static {
        let (_, fut) = self.bind_ephemeral(addr);
        fut
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    pub fn bind_ephemeral(self, addr: impl Into<SocketAddr> + 'static) -> (SocketAddr, impl Future<Item=(), Error=()> + 'static) {
        let (addr, srv) = bind_inner!(self, addr);
        (addr, srv.map_err(|e| error!("server error: {}", e)))
    }

    /// Create a server with graceful shutdown signal.
    ///
    /// When the signal completes, the server will start the graceful shutdown
    /// process.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    /// # Example
    ///
    /// ```no_run
    /// extern crate futures;
    /// extern crate warp;
    ///
    /// use futures::sync::oneshot;
    /// use warp::Filter;
    ///
    /// # fn main() {
    /// let routes = warp::any()
    ///     .map(|| "Hello, World!");
    ///
    /// let (tx, rx) = oneshot::channel();
    ///
    /// let (addr, server) = warp::serve(routes)
    ///     .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), rx);
    ///
    /// // Spawn the server into a runtime
    /// warp::spawn(server);
    ///
    /// // Later, start the shutdown...
    /// let _ = tx.send(());
    /// # }
    /// ```
    pub fn bind_with_graceful_shutdown(
        self,
        addr: impl Into<SocketAddr> + 'static,
        signal: impl Future<Item=()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Item=(), Error=()> + 'static) {
        let (addr, srv) = bind_inner!(self, addr);
        let fut = srv
            .with_graceful_shutdown(signal)
            .map_err(|e| error!("server error: {}", e));
        (addr, fut)
    }


    /// Setup this `Server` with a specific stream of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    ///
    /// Returns a `Future` that can be executed on any runtime.
    pub fn serve_incoming<I>(self, incoming: I) -> impl Future<Item=(), Error=()> + 'static
    where
        I: Stream + Send + 'static,
        I::Item: AsyncRead + AsyncWrite + Send + 'static,
        I::Error: Into<Box<::std::error::Error + Send + Sync>>,
    {
        let service = into_service!(self);
        HyperServer::builder(incoming)
            .http1_pipeline_flush(self.pipeline)
            .serve(service)
            .map_err(|e| error!("server error: {}", e))
    }

    // Generally shouldn't be used, as it can slow down non-pipelined responses.
    //
    // It's only real use is to make silly pipeline benchmarks look better.
    #[doc(hidden)]
    pub fn unstable_pipeline(mut self) -> Self {
        self.pipeline = true;
        self
    }
}

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Future + Send;
    fn call(&self, req: Request) -> Self::Reply;
}


// Optimizes better than using Future::then, since it doesn't
// have to return an IntoFuture.
#[derive(Debug)]
struct ReplyFuture<F> {
    inner: F,
}

impl<F> Future for ReplyFuture<F>
where
    F: Future,
    F::Item: Reply,
    F::Error: Reject,
{
    type Item = ::reply::Response;
    type Error = Never;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(ok)) => Ok(Async::Ready(ok.into_response())),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(err) => {
                debug!("rejected: {:?}", err);
                Ok(Async::Ready(err.into_response()))
            }
        }
    }
}

