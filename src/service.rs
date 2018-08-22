use std::sync::Arc;

use hyper::{service as hyper_service, Body};
use futures::{Future, Poll, Async, future::FutureResult, future};

use ::Request;
use ::never::Never;
use ::reply::{Reply, ReplySealed};
use ::reject::Reject;

#[derive(Debug)]
pub struct HyperService<T> {
    inner: Arc<T>
}

impl<T,R> hyper_service::Service for HyperService<T>
where
    T: WarpService<Reply = R>,
    R: Future + Send,
    R::Item: Reply + Send,
    R::Error: Reject + Send
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Never;
    type Future = ReplyFuture<T::Reply>;

    #[inline]
    fn call(&mut self, req: Request) -> Self::Future {
        let inner = self.inner.clone();
        ReplyFuture {
           inner: inner.call(req)
        }
    }
}

#[derive(Debug)]
pub struct HyperNewService<T> {
    inner: Arc<T>
}

/// Converts given `service` instance into `HyperNewService` factory.
///
/// # Examples
///
/// ```
/// # extern crate hyper;
/// # extern crate warp;
/// use warp::{Future, Filter, new_service};
/// use hyper::Server;
///
/// fn main() {
///   let addr = ([127, 0, 0, 1], 3000).into();
///   let endpoint = warp::any().map(|| "hello");
///   let factory = new_service(endpoint);
///   let server = Server::bind(&addr)
///     .serve(factory)
///     .map_err(|err| panic!("server error: {}", err));
///   # drop(server)
/// }
/// ```
///
pub fn new_service<T>(service: T) -> HyperNewService<T::Service>
where T: IntoWarpService {
    HyperNewService { inner: Arc::new(service.into_warp_service()) }
}

impl<T, R> hyper_service::NewService for HyperNewService<T>
where
    T: WarpService<Reply = R>,
    R: Future + Send,
    R::Item: Reply + Send,
    R::Error: Reject + Send
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Never;
    type Service = HyperService<T>;
    type Future = FutureResult<Self::Service, Self::InitError>;
    type InitError = Never;

    #[inline]
    fn new_service(&self) -> Self::Future {
        let instance = HyperService { inner: self.inner.clone() };
        future::ok(instance)
    }
}

// Optimizes better than using Future::then, since it doesn't
// have to return an IntoFuture.
#[derive(Debug)]
pub struct ReplyFuture<F> {
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

pub trait IntoWarpService {
    type Service: WarpService + Send + Sync + 'static;
    fn into_warp_service(self) -> Self::Service;
}

pub trait WarpService {
    type Reply: Future + Send;
    fn call(&self, req: ::Request) -> Self::Reply;
}
