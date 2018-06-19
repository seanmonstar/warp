use std::net::SocketAddr;
use std::sync::Arc;

use futures::Future;
use http;
use hyper::{rt, Body, Server as HyperServer};
use hyper::service::{service_fn};

use ::never::Never;
use ::reply::{NotFound, Reply, Response, WarpBody};
use ::Request;

pub fn serve<S>(service: S) -> Server<S>
where
    S: IntoWarpService + 'static,
{
    Server {
        service,
    }
}

pub struct Server<S> {
    service: S,
}

impl<S> Server<S>
where
    S: IntoWarpService + 'static,
{
    pub fn run<A>(self, addr: A)
    where
        A: Into<SocketAddr>,
    {
        let inner = Arc::new(self.service.into_warp_service());
        let service = move || {
            let inner = inner.clone();
            service_fn(move |req: ::hyper::Request<Body>| {
                let req: http::Request<Body> = req.into();
                inner.call(req.map(WarpBody::wrap))
                    .into_response()
                    .map(|res: Response| {
                        let res: ::hyper::Response<Body> = res.0.map(WarpBody::unwrap).into();
                        res
                    })
                    .map_err(|x: Never| -> ::hyper::Error { match x {} })
            })
        };
        let srv = HyperServer::bind(&addr.into())
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
