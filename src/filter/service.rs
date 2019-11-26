use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;

use pin_project::pin_project;
use futures::future::TryFuture;

use crate::reject::IsReject;
use crate::reply::Reply;
use crate::route::{self, Route};
use crate::server::{IntoWarpService, WarpService};
use crate::{Filter, Request};

#[derive(Copy, Clone, Debug)]
pub struct FilteredService<F> {
    pub(crate) filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
{
    type Reply = FilteredFuture<F::Future>;

    #[inline]
    fn call(&mut self, req: Request, remote_addr: Option<SocketAddr>) -> Self::Reply {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let route = Route::new(req, remote_addr);
        let fut = route::set(&route, || self.filter.filter());
        FilteredFuture {
            future: fut,
            route,
        }
    }
}

#[pin_project]
#[derive(Debug)]
pub struct FilteredFuture<F> {
    #[pin]
    future: F,
    route: ::std::cell::RefCell<Route>,
}

impl<F> Future for FilteredFuture<F>
where
    F: TryFuture,
    F::Ok: Reply,
    F::Error: IsReject,
{
    type Output = Result<crate::reply::Response, std::convert::Infallible>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let pin = self.project();
        let fut = pin.future;

        match route::set(pin.route, || fut.try_poll(cx)) {
            Poll::Ready(Ok(ok)) => Poll::Ready(Ok(ok.into_response())),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => {
                log::debug!("rejected: {:?}", err);
                Poll::Ready(Ok(err.into_response()))
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TowerService<F> {
    pub(crate) service: F,
}

#[pin_project]
#[derive(Debug)]
pub struct TowerServiceFuture<F> {
    #[pin]
    pub(crate) future: F,
}

impl<S> IntoWarpService for S
where
    S: tower_service::Service<crate::Request, Response = crate::Response> + Send + Sync + 'static,
    S::Error: crate::reject::Reject,
    S::Future: Send,
{
    type Service = TowerService<S>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        TowerService{ service: self }
    }
}

impl<S> WarpService for TowerService<S>
where
    S: tower_service::Service<crate::Request, Response = crate::Response> + Send + Sync + 'static,
    S::Error: crate::reject::Reject,
    S::Future: Send
{
    type Reply = TowerServiceFuture<S::Future>;

    fn call(&mut self, req: Request, _remote_addr: Option<SocketAddr>) -> Self::Reply {

        TowerServiceFuture{ future: self.service.call(req) }
    }
}

impl<S> Future for TowerServiceFuture<S>
where
    S: TryFuture<Ok = crate::Response>,
    S::Error: crate::reject::Reject,
{
    type Output = Result<crate::reply::Response, std::convert::Infallible>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let pin = self.project();
        let fut = pin.future;

        match fut.try_poll(cx) {
            Poll::Ready(Ok(ok)) => Poll::Ready(Ok(ok)),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(err)) => {
                log::debug!("rejected: {:?}", err);
                Poll::Ready(Ok(crate::reject::custom(err).into_response()))
            }
        }
    }
}

impl<F> tower_service::Service<crate::Request> for FilteredService<F>
where
    F: Filter,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
{
    type Response = crate::Response;
    type Error = std::convert::Infallible;
    type Future = FilteredFuture<F::Future>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: crate::Request) -> Self::Future {
        let route = Route::new(req, None);
        let fut = route::set(&route, || self.filter.filter());
        FilteredFuture {
            future: fut,
            route,
        }
    }
}
