use std::net::SocketAddr;
use std::sync::Arc;

use futures::Future;
use http;
use hyper::{rt, Body, Server as HyperServer};
use hyper::service::{service_fn};

use ::reply::{NotFound, Reply};
use ::Request;

/// Create a `Server` with the provided service.
pub fn serve<S>(service: S) -> Server<S>
where
    S: IntoWarpService + 'static,
{
    Server {
        service,
    }
}

/// A Warp Server ready to filter requests.
pub struct Server<S> {
    service: S,
}

impl<S> Server<S>
where
    S: IntoWarpService + 'static,
{
    /// Run this `Server` forever on the current thread.
    pub fn run<A>(self, addr: A)
    where
        A: Into<SocketAddr>,
    {
        let inner = Arc::new(self.service.into_warp_service());
        let service = move || {
            let inner = inner.clone();
            service_fn(move |req: http::Request<Body>| {
                inner.call(req)
                    .into_response()
            })
        };
        let srv = HyperServer::bind(&addr.into())
            .pipeline()
            .serve(service);
        info!("warp drive engaged: listening on {}", srv.local_addr());

        rt::run(srv.map_err(|e| error!("server error: {}", e)));
    }
}

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Reply;
    fn call(&self, req: Request) -> Self::Reply;
}

/*
impl<T> IntoWarpService for T
where
    T: WarpService + Send + Sync + 'static,
{
    type Service = T;

    fn into_warp_service(self) -> Self::Service {
        self
    }
}
*/

impl WarpService for NotFound {
    type Reply = NotFound;

    fn call(&self, _: Request) -> Self::Reply {
        *self
    }
}
