use futures::Future;

use ::{Filter, Request};
use ::reply::{Reply};
use ::route::Route;
use ::server::{IntoWarpService, WarpService};

#[derive(Copy, Clone, Debug)]
pub struct FilteredService<F> {
    filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reply,
{
    type Reply = F::Future;

    #[inline]
    fn call(&self, req: Request) -> Self::Reply {
        self.filter.filter(Route::new(req))
    }
}

impl<F> IntoWarpService for FilteredService<F>
where
    F: Filter + Send + Sync + 'static,
    <F::Future as Future>::Item: Reply,
    <F::Future as Future>::Error: Reply,
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
    <F::Future as Future>::Error: Reply,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService {
            filter: self,
        }
    }
}

