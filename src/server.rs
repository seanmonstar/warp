use std::net::SocketAddr;
use std::sync::Arc;

use futures::Future;
use hyper::{rt, Server as HyperServer};

use ::filter::Filter;
use ::never::Never;
use ::reject::Reject;
use ::reply::Reply;
use ::Request;

/// Create a `Server` with the provided service.
pub fn serve<F>(filter: F) -> Server<F>
where
    F: Filter + Send + Sync + 'static,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    Server {
        pipeline: false,
        filter,
    }
}

/// A Warp Server ready to filter requests.
#[derive(Debug)]
pub struct Server<F> {
    pipeline: bool,
    filter: F,
}

impl<F> Server<F>
where
    F: Filter + Send + Sync + 'static,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    /// Run this `Server` forever on the current thread.
    pub fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, future) = self.bind_ephemeral(addr);

        info!("warp drive engaged: listening on {}", addr);

        rt::run(future);
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    pub fn bind(self, addr: impl Into<SocketAddr> + 'static) -> impl Future<Item=(), Error=()> + 'static {
        let (_, future) = self.bind_ephemeral(addr);
        future
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    pub fn bind_ephemeral(self, addr: impl Into<SocketAddr> + 'static) -> (SocketAddr, impl Future<Item=(), Error=()> + 'static) {
        let inner = Arc::new(self.filter);
        let new_service = move || Ok::<_, Never>(inner.clone().lift());
        let srv = HyperServer::bind(&addr.into())
            .http1_pipeline_flush(self.pipeline)
            .serve(new_service);
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

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Future + Send;
    fn call(&self, req: Request) -> Self::Reply;
}

