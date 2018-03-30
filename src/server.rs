use std::net::SocketAddr;

use http::{Request};
//use hyper::server::Service;
use hyper::Body;
use hyper::server::{Http, const_service, service_fn};

use ::reply::{Reply, WarpBody};

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
        let inner = self.service.into_warp_service();;
        let service = const_service(service_fn(move |req: ::hyper::Request<Body>| {
            let req: Request<Body> = req.into();
            let res = inner.call(req.map(WarpBody)).into_response();
            let res: ::hyper::Response<Body> = res.map(|w| w.0).into();
            Ok(res)
        }));
        Http::new()
            .bind(&addr.into(), service)
            .expect("error binding to address")
            .run()
            .expect("error running server");
    }
}

pub trait IntoWarpService {
    type Service: WarpService;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Reply;
    fn call(&self, req: Request<WarpBody>) -> Self::Reply;
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
    type Reply = &'static str;
    fn call(&self, _: Request<WarpBody>) -> Self::Reply {
        (*self)()
    }
}
