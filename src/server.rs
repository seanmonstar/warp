use std::future::Future;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;

use futures_util::TryFuture;

use crate::filter::Filter;
use crate::reject::IsReject;
use crate::reply::Reply;
#[cfg(feature = "tls")]
use crate::tls::TlsConfigBuilder;

/// Create a `Server` with the provided `Filter`.
pub fn serve<F>(filter: F) -> Server<F, accept::LazyTcp, run::Standard>
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: IsReject,
{
    Server {
        acceptor: accept::LazyTcp,
        pipeline: false,
        filter,
        runner: run::Standard,
    }
}

/// A warp Server ready to filter requests.
///
/// Construct this type using [`serve()`].
///
/// # Unnameable
///
/// This type is publicly available in the docs only.
///
/// It is not otherwise nameable, since it is a builder type using typestate
/// to allow for ergonomic configuration.
#[derive(Debug)]
pub struct Server<F, A, R> {
    acceptor: A,
    filter: F,
    pipeline: bool,
    runner: R,
}

// ===== impl Server =====

impl<F, R> Server<F, accept::LazyTcp, R>
where
    F: Filter + Clone + Send + Sync + 'static,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
    R: run::Run,
{
    /// Binds and runs this server.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    ///
    /// To handle bind failures, bind a listener and call `incoming()`.
    pub async fn run(self, addr: impl Into<SocketAddr>) {
        self.bind(addr).await.run().await;
    }

    /// Binds this server.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    ///
    /// To handle bind failures, bind a listener and call `incoming()`.
    pub async fn bind(self, addr: impl Into<SocketAddr>) -> Server<F, tokio::net::TcpListener, R> {
        let addr = addr.into();
        let acceptor = tokio::net::TcpListener::bind(addr)
            .await
            .expect("failed to bind to address");

        self.incoming(acceptor)
    }

    /// Configure the server with an acceptor of incoming connections.
    pub fn incoming<A>(self, acceptor: A) -> Server<F, A, R> {
        Server {
            acceptor,
            filter: self.filter,
            pipeline: self.pipeline,
            runner: self.runner,
        }
    }

    // pub fn conn
}

impl<F, A, R> Server<F, A, R>
where
    F: Filter + Clone + Send + Sync + 'static,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
    A: accept::Accept,
    R: run::Run,
{
    #[cfg(feature = "tls")]
    pub fn tls(self) -> Server<F, accept::Tls<A>, R> {}

    /// Add graceful shutdown support to this server.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn ex(addr: std::net::SocketAddr) {
    /// # use warp::Filter;
    /// # let filter = warp::any().map(|| "ok");
    /// warp::serve(filter)
    ///     .bind(addr).await
    ///     .graceful(async {
    ///         // some signal in here, such as ctrl_c
    ///     })
    ///     .run().await;
    /// # }
    /// ```
    pub fn graceful<Fut>(self, shutdown_signal: Fut) -> Server<F, A, run::Graceful<Fut>>
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        Server {
            acceptor: self.acceptor,
            filter: self.filter,
            pipeline: self.pipeline,
            runner: run::Graceful(shutdown_signal),
        }
    }

    /// Run this server.
    pub async fn run(self) {
        R::run(self).await;
    }
}

// // ===== impl Tls =====

#[cfg(feature = "tls")]
impl<F, A, R> Server<F, accept::Tls<A>, R>
where
    F: Filter + Clone + Send + Sync + 'static,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
    A: accept::Accept,
    R: run::Run,
{
    // TLS config methods

    /// Specify the file path to read the private key.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn key_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.key_path(path))
    }

    /// Specify the file path to read the certificate.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn cert_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.cert_path(path))
    }

    /// Specify the file path to read the trust anchor for optional client authentication.
    ///
    /// Anonymous and authenticated clients will be accepted. If no trust anchor is provided by any
    /// of the `client_auth_` methods, then client authentication is disabled by default.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn client_auth_optional_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.client_auth_optional_path(path))
    }

    /// Specify the file path to read the trust anchor for required client authentication.
    ///
    /// Only authenticated clients will be accepted. If no trust anchor is provided by any of the
    /// `client_auth_` methods, then client authentication is disabled by default.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn client_auth_required_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.client_auth_required_path(path))
    }

    /// Specify the in-memory contents of the private key.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn key(self, key: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.key(key.as_ref()))
    }

    /// Specify the in-memory contents of the certificate.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn cert(self, cert: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.cert(cert.as_ref()))
    }

    /// Specify the in-memory contents of the trust anchor for optional client authentication.
    ///
    /// Anonymous and authenticated clients will be accepted. If no trust anchor is provided by any
    /// of the `client_auth_` methods, then client authentication is disabled by default.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn client_auth_optional(self, trust_anchor: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.client_auth_optional(trust_anchor.as_ref()))
    }

    /// Specify the in-memory contents of the trust anchor for required client authentication.
    ///
    /// Only authenticated clients will be accepted. If no trust anchor is provided by any of the
    /// `client_auth_` methods, then client authentication is disabled by default.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn client_auth_required(self, trust_anchor: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.client_auth_required(trust_anchor.as_ref()))
    }

    /// Specify the DER-encoded OCSP response.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn ocsp_resp(self, resp: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.ocsp_resp(resp.as_ref()))
    }

    fn with_tls<Func>(self, func: Func) -> Self
    where
        Func: FnOnce(TlsConfigBuilder) -> TlsConfigBuilder,
    {
        let tls = func(tls);
    }
}

mod accept {
    pub trait Accept {
        type IO: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static;
        type AcceptError: std::fmt::Debug;
        type Accepting: super::Future<Output = Result<Self::IO, Self::AcceptError>> + Send + 'static;
        #[allow(async_fn_in_trait)]
        async fn accept(&mut self) -> Result<Self::Accepting, std::io::Error>;
    }

    #[derive(Debug)]
    pub struct LazyTcp;

    impl Accept for tokio::net::TcpListener {
        type IO = hyper_util::rt::TokioIo<tokio::net::TcpStream>;
        type AcceptError = std::convert::Infallible;
        type Accepting = std::future::Ready<Result<Self::IO, Self::AcceptError>>;
        async fn accept(&mut self) -> Result<Self::Accepting, std::io::Error> {
            let (io, _addr) = <tokio::net::TcpListener>::accept(self).await?;
            Ok(std::future::ready(Ok(hyper_util::rt::TokioIo::new(io))))
        }
    }

    #[cfg(unix)]
    impl Accept for tokio::net::UnixListener {
        type IO = hyper_util::rt::TokioIo<tokio::net::UnixStream>;
        type AcceptError = std::convert::Infallible;
        type Accepting = std::future::Ready<Result<Self::IO, Self::AcceptError>>;
        async fn accept(&mut self) -> Result<Self::Accepting, std::io::Error> {
            let (io, _addr) = <tokio::net::UnixListener>::accept(self).await?;
            Ok(std::future::ready(Ok(hyper_util::rt::TokioIo::new(io))))
        }
    }

    #[cfg(feature = "tls")]
    #[derive(Debug)]
    pub struct Tls<A>(pub(super) A);

    #[cfg(feature = "tls")]
    impl<A: Accept> Accept for Tls<A> {
        type IO = hyper_util::rt::TokioIo<tokio::net::TcpStream>;
        type AcceptError = std::convert::Infallible;
        type Accepting = std::future::Ready<Result<Self::IO, Self::AcceptError>>;
        async fn accept(&mut self) -> Result<Self::Accepting, std::io::Error> {
            let (io, _addr) = self.0.accept().await?;
            Ok(std::future::ready(Ok(hyper_util::rt::TokioIo::new(io))))
        }
    }
}

mod run {
    pub trait Run {
        #[allow(async_fn_in_trait)]
        async fn run<F, A>(server: super::Server<F, A, Self>)
        where
            F: super::Filter + Clone + Send + Sync + 'static,
            <F::Future as super::TryFuture>::Ok: super::Reply,
            <F::Future as super::TryFuture>::Error: super::IsReject,
            A: super::accept::Accept,
            Self: Sized;
    }

    #[derive(Debug)]
    pub struct Standard;

    impl Run for Standard {
        async fn run<F, A>(mut server: super::Server<F, A, Self>)
        where
            F: super::Filter + Clone + Send + Sync + 'static,
            <F::Future as super::TryFuture>::Ok: super::Reply,
            <F::Future as super::TryFuture>::Error: super::IsReject,
            A: super::accept::Accept,
            Self: Sized,
        {
            let pipeline = server.pipeline;
            loop {
                let accepting = match server.acceptor.accept().await {
                    Ok(fut) => fut,
                    Err(err) => {
                        handle_accept_error(err).await;
                        continue;
                    }
                };
                let svc = crate::service(server.filter.clone());
                let svc = hyper_util::service::TowerToHyperService::new(svc);
                tokio::spawn(async move {
                    let io = match accepting.await {
                        Ok(io) => io,
                        Err(err) => {
                            tracing::debug!("server accept error: {:?}", err);
                            return;
                        }
                    };
                    if let Err(err) = hyper_util::server::conn::auto::Builder::new(
                        hyper_util::rt::TokioExecutor::new(),
                    )
                    .http1()
                    .pipeline_flush(pipeline)
                    .serve_connection_with_upgrades(io, svc)
                    .await
                    {
                        tracing::error!("server connection error: {:?}", err)
                    }
                });
            }
        }
    }

    #[derive(Debug)]
    pub struct Graceful<Fut>(pub(super) Fut);

    impl<Fut> Run for Graceful<Fut>
    where
        Fut: super::Future<Output = ()> + Send + 'static,
    {
        async fn run<F, A>(mut server: super::Server<F, A, Self>)
        where
            F: super::Filter + Clone + Send + Sync + 'static,
            <F::Future as super::TryFuture>::Ok: super::Reply,
            <F::Future as super::TryFuture>::Error: super::IsReject,
            A: super::accept::Accept,
            Self: Sized,
        {
            use futures_util::future;

            let pipeline = server.pipeline;
            let graceful_util = hyper_util::server::graceful::GracefulShutdown::new();
            let mut shutdown_signal = std::pin::pin!(server.runner.0);
            loop {
                let accept = std::pin::pin!(server.acceptor.accept());
                let accepting = match future::select(accept, &mut shutdown_signal).await {
                    future::Either::Left((Ok(fut), _)) => fut,
                    future::Either::Left((Err(err), _)) => {
                        handle_accept_error(err).await;
                        continue;
                    }
                    future::Either::Right(((), _)) => {
                        tracing::debug!("shutdown signal received, starting graceful shutdown");
                        break;
                    }
                };
                let svc = crate::service(server.filter.clone());
                let svc = hyper_util::service::TowerToHyperService::new(svc);
                let watcher = graceful_util.watcher();
                tokio::spawn(async move {
                    let io = match accepting.await {
                        Ok(io) => io,
                        Err(err) => {
                            tracing::debug!("server accepting error: {:?}", err);
                            return;
                        }
                    };
                    let mut hyper = hyper_util::server::conn::auto::Builder::new(
                        hyper_util::rt::TokioExecutor::new(),
                    );
                    hyper.http1().pipeline_flush(pipeline);
                    let conn = hyper.serve_connection_with_upgrades(io, svc);
                    let conn = watcher.watch(conn);
                    if let Err(err) = conn.await {
                        tracing::error!("server connection error: {:?}", err)
                    }
                });
            }

            drop(server.acceptor); // close listener
            graceful_util.shutdown().await;
        }
    }

    // TODO: allow providing your own handler
    async fn handle_accept_error(e: std::io::Error) {
        if is_connection_error(&e) {
            return;
        }
        // [From `hyper::Server` in 0.14](https://github.com/hyperium/hyper/blob/v0.14.27/src/server/tcp.rs#L186)
        //
        // > A possible scenario is that the process has hit the max open files
        // > allowed, and so trying to accept a new connection will fail with
        // > `EMFILE`. In some cases, it's preferable to just wait for some time, if
        // > the application will likely close some files (or connections), and try
        // > to accept the connection again. If this option is `true`, the error
        // > will be logged at the `error` level, since it is still a big deal,
        // > and then the listener will sleep for 1 second.
        tracing::error!("accept error: {:?}", e);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    fn is_connection_error(e: &std::io::Error) -> bool {
        // some errors that occur on the TCP stream are emitted when
        // accepting, they can be ignored.
        matches!(
            e.kind(),
            std::io::ErrorKind::ConnectionRefused
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::ConnectionReset
        )
    }
}
