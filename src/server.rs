use std::net::SocketAddr;

use futures::Future;
use http;
//use hyper::server::Service;
use hyper::Body;
use hyper::server::{Http, const_service, service_fn};

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
        let inner = self.service.into_warp_service();
        let service = const_service(service_fn(move |req: ::hyper::Request<Body>| {
            let req: http::Request<Body> = req.into();
            inner.call(req.map(WarpBody::wrap))
                .into_response()
                .map(|res: Response| {
                    let res: ::hyper::Response<Body> = res.0.map(WarpBody::unwrap).into();
                    res
                })
                .map_err(|x: !| -> ::hyper::Error { x })
        }));
        let srv = Http::new()
            .bind(&addr.into(), service)
            .expect("error binding to address");
        info!("warp drive engaged: listening on {}", srv.local_addr().unwrap());

        srv.run()
            .expect("error running server");
    }
}

pub trait IntoWarpService {
    type Service: WarpService;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Reply;
    fn call(&self, req: Request) -> Self::Reply;
}

impl<T> IntoWarpService for T
where
    T: WarpService,
{
    type Service = T;

    fn into_warp_service(self) -> Self::Service {
        self
    }
}

impl<T> WarpService for T
where
    T: Fn() -> &'static str,
{
    type Reply = Response;

    fn call(&self, _: Request) -> Self::Reply {
        (*self)().into()
    }
}

impl WarpService for NotFound {
    type Reply = NotFound;

    fn call(&self, _: Request) -> Self::Reply {
        *self
    }
}
