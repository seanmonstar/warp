use std::net::SocketAddr;
use std::sync::Arc;

use futures::{Async, Future, Poll};
use hyper::{rt, Server as HyperServer};
use hyper::service::{service_fn};

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
    pub fn run<A>(self, addr: A)
    where
        A: Into<SocketAddr>,
    {
        let inner = Arc::new(self.service.into_warp_service());
        let service = move || {
            let inner = inner.clone();
            service_fn(move |req| {
                ReplyFuture {
                    inner: inner.call(req)
                }
            })
        };
        let srv = HyperServer::bind(&addr.into())
            .http1_pipeline_flush(self.pipeline)
            .serve(service);
        info!("warp drive engaged: listening on {}", srv.local_addr());

        rt::run(srv.map_err(|e| error!("server error: {}", e)));
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
            Err(err) => Ok(Async::Ready(err.into_response())),
        }
    }
}

