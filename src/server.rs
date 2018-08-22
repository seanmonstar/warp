use std::net::SocketAddr;

use futures::Future;
use hyper::{rt, Server as HyperServer};

use ::reject::Reject;
use ::reply::Reply;
use ::service::{IntoWarpService, WarpService, new_service};

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
        let factory = new_service(self.service);
        let srv = HyperServer::bind(&addr.into())
            .http1_pipeline_flush(self.pipeline)
            .serve(factory);
        let addr = srv.local_addr();
        (addr, srv.map_err(|e| error!("server error: {}", e)))
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

