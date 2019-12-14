use std::error::Error as StdError;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;
use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;
use std::convert::Infallible;

use pin_project::pin_project;
use futures::{future, FutureExt, TryFuture, TryStream, TryStreamExt};
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Server as HyperServer};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::reject::IsReject;
use crate::reply::Reply;
use crate::transport::Transport;
use crate::Request;

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
            future::ok::<_, hyper::Error>(service_fn(move |req| ReplyFuture {
                inner: inner.call(req, remote_addr),
            }))
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
        let srv = HyperServer::builder(crate::tls::TlsAcceptor::new($this.tls, incoming))
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
    <<S::Service as WarpService>::Reply as TryFuture>::Ok: Reply + Send,
    <<S::Service as WarpService>::Reply as TryFuture>::Error: IsReject + Send,
{
    /// Run this `Server` forever on the current thread.
    pub async fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);


        log::info!("warp drive engaged: listening on http://{}", addr);

        fut.await;
    }

    /// Run this `Server` forever on the current thread with a specific stream
    /// of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    pub async fn run_incoming<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: AsyncRead + AsyncWrite + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        self.run_incoming2(incoming.map_ok(crate::transport::LiftIo).into_stream()).await;
    }

    async fn run_incoming2<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: Transport + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let fut = self.serve_incoming2(incoming);

        log::info!("warp drive engaged: listening with custom incoming");

        fut.await;
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
    ) -> impl Future<Output = ()> + 'static {
        let (_, fut) = self.bind_ephemeral(addr);
        fut
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// In case we are unable to bind to the specified address, resolves to an
    /// error and logs the reason.
    pub async fn try_bind(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) {
        let addr = addr.into();
        let srv = match try_bind!(self, &addr) {
            Ok((_, srv)) => srv,
            Err(err) => {
                log::error!("error binding to {}: {}", addr, err);
                return;
            }
        };

        srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        }).await;
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
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        (addr, srv)
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
    ) -> Result<(SocketAddr, impl Future<Output = ()> + 'static), hyper::error::Error>
    {
        let addr = addr.into();
        let (addr, srv) = try_bind!(self, &addr)?;
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        Ok((addr, srv))
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
    /// use warp::Filter;
    /// use futures::future::TryFutureExt;
    /// use tokio::sync::oneshot;
    ///
    /// # fn main() {
    /// let routes = warp::any()
    ///     .map(|| "Hello, World!");
    ///
    /// let (tx, rx) = oneshot::channel();
    ///
    /// let (addr, server) = warp::serve(routes)
    ///     .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
    ///          rx.await.ok();
    ///     });
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
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
        let fut = srv
            .with_graceful_shutdown(signal)
            .map(|result| {
                if let Err(err) = result {
                    log::error!("server error: {}", err)
                }
            });
        (addr, fut)
    }

    /// Setup this `Server` with a specific stream of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    ///
    /// Returns a `Future` that can be executed on any runtime.
    pub fn serve_incoming<I>(self, incoming: I) -> impl Future<Output = ()>
    where
        I: TryStream + Send,
        I::Ok: AsyncRead + AsyncWrite + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let incoming = incoming.map_ok(crate::transport::LiftIo);
        self.serve_incoming2(incoming)
    }

    async fn serve_incoming2<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: Transport + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let service = into_service!(self.service);

        let srv = HyperServer::builder(hyper::server::accept::from_stream(incoming.into_stream()))
            .http1_pipeline_flush(self.pipeline)
            .serve(service).await;

        if let Err(err) = srv {
            log::error!("server error: {}", err);
        }
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
        let tls = crate::tls::configure(cert.as_ref(), key.as_ref());

        TlsServer { server: self, tls }
    }
}

// // ===== impl TlsServer =====

#[cfg(feature = "tls")]
impl<S> TlsServer<S>
where
    S: IntoWarpService + 'static,
    <<S::Service as WarpService>::Reply as TryFuture>::Ok: Reply + Send,
    <<S::Service as WarpService>::Reply as TryFuture>::Error: IsReject + Send,
{
    /// Run this `TlsServer` forever on the current thread.
    ///
    /// *This function requires the `"tls"` feature.*
    pub async fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        log::info!("warp drive engaged: listening on https://{}", addr);

        fut.await;
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// *This function requires the `"tls"` feature.*
    pub async fn bind(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) {
        let (_, fut) = self.bind_ephemeral(addr);
        fut.await;
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
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        (addr, srv)
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
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);

        let fut = srv
            .with_graceful_shutdown(signal)
            .map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }});
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

// // ===== impl WarpService =====

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: TryFuture + Send;
    fn call(&self, req: Request, remote_addr: Option<SocketAddr>) -> Self::Reply;
}

// Optimizes better than using Future::then, since it doesn't
// have to return an IntoFuture.
#[pin_project]
#[derive(Debug)]
struct ReplyFuture<F> {
    #[pin]
    inner: F,
}

impl<F> Future for ReplyFuture<F>
where
    F: TryFuture,
    F::Ok: Reply,
    F::Error: IsReject,
{
    type Output = Result<crate::reply::Response, Infallible>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match pin.inner.try_poll(cx) {
            Poll::Ready(Ok(ok)) => Poll::Ready(Ok(ok.into_response())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => {
                log::debug!("rejected: {:?}", err);
                Poll::Ready(Ok(err.into_response()))
            }
        }
    }
}
