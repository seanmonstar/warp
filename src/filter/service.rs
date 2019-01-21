use std::net::SocketAddr;

use futures::{Future, Poll};

use reject::Reject;
use reply::Reply;
use route::{self, Route};
use server::{IntoWarpService, WarpService};
use {Filter, Request};

#[derive(Copy, Clone, Debug)]
pub struct FilteredService<F> {
    filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    type Reply = FilteredFuture<F::Future>;

    #[inline]
    fn call(&self, req: Request, remote_addr: Option<SocketAddr>) -> Self::Reply {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let route = Route::new(req, remote_addr);
        let fut = route::set(&route, || self.filter.filter());
        FilteredFuture {
            future: fut,
            route: route,
        }
    }
}

#[derive(Debug)]
pub struct FilteredFuture<F> {
    future: F,
    route: ::std::cell::RefCell<Route>,
}

impl<F> Future for FilteredFuture<F>
where
    F: Future,
{
    type Item = F::Item;
    type Error = F::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        debug_assert!(!route::is_set(), "nested route::set calls");

        let fut = &mut self.future;
        route::set(&self.route, || fut.poll())
    }
}

impl<F> IntoWarpService for FilteredService<F>
where
    F: Filter + Send + Sync + 'static,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
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
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reject,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService { filter: self }
    }
}
