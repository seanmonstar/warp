use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;

use pin_project::pin_project;
use futures::future::TryFuture;

use crate::reject::Reject;
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
    <F::Future as TryFuture>::Error: Reject,
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
{
    type Output = Result<F::Ok, F::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let pin = self.project();
        let fut = pin.future;
        route::set(&pin.route, || fut.try_poll(cx))
    }
}

impl<F> IntoWarpService for FilteredService<F>
where
    F: Filter + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: Reject,
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
    F::Error: Reject,
    {
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService { filter: self }
    }
}
