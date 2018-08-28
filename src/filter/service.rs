use std::cell::RefCell;

use futures::{Future, Async, Poll};
use tower_service::Service as TowerService;
use hyper::service::Service as HyperService;
use hyper::Body;

use ::filter::Filter;
use ::Request;
use ::reply::{Reply, Response, ReplySealed};
use ::reject::Reject;
use ::never::Never;
use ::route::{self, Route};

/// Wraps a `Filter` instance, implementing `tower_service::Service` and `hyper::service::Service`.
#[derive(Debug)]
pub struct FilterService<F> {
    filter: F
}

/// Wraps a `Filter` instance, implementing `tower_service::Service` and `hyper::service::Service`.
pub fn service<F>(filter: F) -> FilterService<F>
where
    F: Filter + Sized
{
    FilterService { filter }
}

impl<F> TowerService for FilterService<F>
where
    F: Filter + Sized,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    type Request = Request;
    type Response = Response;
    type Error = Never;
    type Future = ResponseFuture<F::Future>;

    #[inline]
    fn poll_ready(&mut self) -> Result<Async<()>, Self::Error> {
        Ok(Async::Ready(()))
    }

    #[inline]
    fn call(&mut self, req: Self::Request) -> Self::Future {
        response_future(req, &self.filter)
    }
}

impl<F> HyperService for FilterService<F>
where
    F: Filter + Sized,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    type ReqBody = Body;
    type ResBody = Body;
    type Error = Never;
    type Future = ResponseFuture<F::Future>;

    #[inline]
    fn call(&mut self, req: Request) -> Self::Future {
        response_future(req, &self.filter)
    }
}

#[derive(Debug)]
pub struct ResponseFuture<F> {
    future: F,
    route: RefCell<Route>,
}

#[inline]
fn response_future<F>(req: Request, filter: &F) -> ResponseFuture<F::Future>
where
    F: Filter + Sized,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject
{
    debug_assert!(!route::is_set(), "nested route::set calls");

    let route = Route::new(req);
    let future = route::set(&route, || filter.filter());

    ResponseFuture {
        route,
        future,
    }
}

impl<F> Future for ResponseFuture<F>
where
    F: Future,
    F::Item: Reply,
    F::Error: Reject,
{
    type Item = Response;
    type Error = Never;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let future = &mut self.future;
        match route::set(&self.route, || future.poll()) {
            Ok(Async::Ready(ok)) => Ok(Async::Ready(ok.into_response())),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(err) => Ok(Async::Ready(err.into_response())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::prelude::Async;
    use hyper::StatusCode;

    use ::filters::any::any;
    use ::reject;

    #[test]
    fn handle_reply() {
        let filter = any().map(|| "ok");
        let ret = match response_future(Default::default(), &filter).poll() {
            Ok(Async::Ready(ok)) => ok,
            _ => unreachable!()
        };
        assert_eq!(200, ret.status());
    }

    #[test]
    fn handle_reject() {
        let filter = any().and_then(|| Err::<StatusCode, _>(reject::server_error()));
        let ret = match response_future(Default::default(), &filter).poll() {
            Ok(Async::Ready(ok)) => ok,
            _ => unreachable!()
        };
        assert_eq!(500, ret.status());
    }
}

