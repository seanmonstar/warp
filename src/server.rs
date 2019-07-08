use std::error::Error as StdError;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;
use std::sync::Arc;

use futures::{Async, Future, Poll, Stream};
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::{rt, Server as HyperServer};
use tokio_io::{AsyncRead, AsyncWrite};

use never::Never;
use reject::Reject;
use reply::Reply;
use transport::Transport;
use Request;

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

/// A Warp Server ready to filter requests over TLS.
///
/// *This type requires the `"tls"` feature.*
#[cfg(feature = "tls")]
pub struct TlsServer<S> {
    server: Server<S>,
    tls: ::rustls::ServerConfig,
}

// Getting all various generic bounds to make this a re-usable method is
// very complicated, so instead this is just a macro.
macro_rules! into_service {
    ($into:expr) => {{
        let inner = Arc::new($into.into_warp_service());
        make_service_fn(move |transport| {
            let inner = inner.clone();
            let remote_addr = Transport::remote_addr(transport);
            service_fn(move |req| ReplyFuture {
                inner: inner.call(req, remote_addr),
            })
        })
    }};
}

macro_rules! addr_incoming {
    ($addr:expr) => {{
        let mut incoming = AddrIncoming::bind($addr)?;
        incoming.set_nodelay(true);
        let addr = incoming.local_addr();
        (addr, incoming)
    }};
}

macro_rules! bind_inner {
    ($this:ident, $addr:expr) => {{
        let service = into_service!($this.service);
        let (addr, incoming) = addr_incoming!($addr);
        let srv = HyperServer::builder(incoming)
            .http1_pipeline_flush($this.pipeline)
            .serve(service);
        Ok::<_, hyper::error::Error>((addr, srv))
    }};

    (tls: $this:ident, $addr:expr) => {{
        let service = into_service!($this.server.service);
        let (addr, incoming) = addr_incoming!($addr);
        let tls = Arc::new($this.tls);
        let incoming = incoming.map(move |sock| {
            let session = ::rustls::ServerSession::new(&tls);
            ::tls::TlsStream::new(sock, session)
        });
        let srv = HyperServer::builder(incoming)
            .http1_pipeline_flush($this.server.pipeline)
            .serve(service);
        Ok::<_, hyper::error::Error>((addr, srv))
    }};
}

macro_rules! bind {
    ($this:ident, $addr:expr) => {{
        let addr = $addr.into();
        (|addr| bind_inner!($this, addr))(&addr).unwrap_or_else(|e| {
            panic!("error binding to {}: {}", addr, e);
        })
    }};

    (tls: $this:ident, $addr:expr) => {{
        let addr = $addr.into();
        (|addr| bind_inner!(tls: $this, addr))(&addr).unwrap_or_else(|e| {
            panic!("error binding to {}: {}", addr, e);
        })
    }};
}

macro_rules! try_bind {
    ($this:ident, $addr:expr) => {{
        (|addr| bind_inner!($this, addr))($addr)
    }};

    (tls: $this:ident, $addr:expr) => {{
        (|addr| bind_inner!(tls: $this, addr))($addr)
    }};
}

// ===== impl Server =====

impl<S> Server<S>
where
    S: IntoWarpService + 'static,
    <<S::Service as WarpService>::Reply as Future>::Item: Reply + Send,
    <<S::Service as WarpService>::Reply as Future>::Error: Reject + Send,
{
    /// Run this `Server` forever on the current thread.
    pub fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        info!("warp drive engaged: listening on http://{}", addr);

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
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        self.run_incoming2(incoming.map(::transport::LiftIo));
    }

    fn run_incoming2<I>(self, incoming: I)
    where
        I: Stream + Send + 'static,
        I::Item: Transport + Send + 'static,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let fut = self.serve_incoming2(incoming);

        info!("warp drive engaged: listening with custom incoming");

        rt::run(fut);
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    pub fn bind(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> impl Future<Item = (), Error = ()> + 'static {
        let (_, fut) = self.bind_ephemeral(addr);
        fut
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// In case we are unable to bind to the specified address, resolves to an
    /// error and logs the reason.
    pub fn try_bind(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> impl Future<Item = (), Error = ()> + 'static {
        let addr = addr.into();
        let result = try_bind!(self, &addr).map_err(|e| error!("error binding to {}: {}", addr, e));
        futures::future::result(result).and_then(|(_, srv)| {
            srv.map_err(|e| error!("server error: {}", e))
        })
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    pub fn bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> (SocketAddr, impl Future<Item = (), Error = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
        (addr, srv.map_err(|e| error!("server error: {}", e)))
    }

    /// Tried to bind a possibly ephemeral socket address.
    ///
    /// Returns a `Result` which fails in case we are unable to bind with the
    /// underlying error.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    pub fn try_bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> Result<(SocketAddr, impl Future<Item = (), Error = ()> + 'static), hyper::error::Error> {
        let addr = addr.into();
        let (addr, srv) = try_bind!(self, &addr)?;
        Ok((addr, srv.map_err(|e| error!("server error: {}", e))))
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
        signal: impl Future<Item = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Item = (), Error = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
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
    pub fn serve_incoming<I>(self, incoming: I) -> impl Future<Item = (), Error = ()> + 'static
    where
        I: Stream + Send + 'static,
        I::Item: AsyncRead + AsyncWrite + Send + 'static,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let incoming = incoming.map(::transport::LiftIo);
        self.serve_incoming2(incoming)
    }

    fn serve_incoming2<I>(self, incoming: I) -> impl Future<Item = (), Error = ()> + 'static
    where
        I: Stream + Send + 'static,
        I::Item: Transport + Send + 'static,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let service = into_service!(self.service);
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

    /// Configure a server to use TLS with the supplied certificate and key files.
    ///
    /// *This function requires the `"tls"` feature.*
    #[cfg(feature = "tls")]
    pub fn tls(self, cert: impl AsRef<Path>, key: impl AsRef<Path>) -> TlsServer<S> {
        let tls = ::tls::configure(cert.as_ref(), key.as_ref());

        TlsServer { server: self, tls }
    }
}

// ===== impl TlsServer =====

#[cfg(feature = "tls")]
impl<S> TlsServer<S>
where
    S: IntoWarpService + 'static,
    <<S::Service as WarpService>::Reply as Future>::Item: Reply + Send,
    <<S::Service as WarpService>::Reply as Future>::Error: Reject + Send,
{
    /// Run this `TlsServer` forever on the current thread.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        info!("warp drive engaged: listening on https://{}", addr);

        rt::run(fut);
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn bind(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> impl Future<Item = (), Error = ()> + 'static {
        let (_, fut) = self.bind_ephemeral(addr);
        fut
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> (SocketAddr, impl Future<Item = (), Error = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);
        (addr, srv.map_err(|e| error!("server error: {}", e)))
    }

    /// Create a server with graceful shutdown signal.
    ///
    /// When the signal completes, the server will start the graceful shutdown
    /// process.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn bind_with_graceful_shutdown(
        self,
        addr: impl Into<SocketAddr> + 'static,
        signal: impl Future<Item = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Item = (), Error = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);

        let fut = srv
            .with_graceful_shutdown(signal)
            .map_err(|e| error!("server error: {}", e));
        (addr, fut)
    }
}

#[cfg(feature = "tls")]
impl<S> ::std::fmt::Debug for TlsServer<S>
where
    S: ::std::fmt::Debug,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct("TlsServer")
            .field("server", &self.server)
            .finish()
    }
}

// ===== impl WarpService =====

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Future + Send;
    fn call(&self, req: Request, remote_addr: Option<SocketAddr>) -> Self::Reply;
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
