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
    filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    <F::Future as TryFuture>::Ok: Reply,
    <F::Future as TryFuture>::Error: IsReject,
{
    type Reply = FilteredFuture<F::Future>;

    #[inline]
    fn call(&self, req: Request, remote_addr: Option<SocketAddr>) -> Self::Reply {
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

impl<F> IntoWarpService for FilteredService<F>
where
    F: Filter + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        self
    }
}

impl<F> IntoWarpService for F
where
    F: Filter + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService { filter: self }
    }
}
