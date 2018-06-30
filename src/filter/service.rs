use futures::future::Either;

use ::{Filter, Request};
use ::reply::{NOT_FOUND, NotFound, Reply};
use ::route::{self, Route};
use ::server::{IntoWarpService, WarpService};

#[derive(Copy, Clone, Debug)]
pub struct FilteredService<F> {
    filter: F,
}

impl<F, R> WarpService for FilteredService<F>
where
    F: Filter<Extract=R>,
    R: Reply,
{
    type Reply = Either<R::Future, <NotFound as Reply>::Future>;
    //type Reply = R;

    #[inline]
    fn call(&self, req: Request) -> Self::Reply {
        debug_assert!(!route::is_set(), "nested FilteredService::calls");

        //let r = Route::new(req);
        //route::set(&r, || {
            self.filter.filter()
        //})
            .map(|reply| {
                Either::A(reply.into_response())
            })
            .unwrap_or_else(|| Either::B(NOT_FOUND.into_response()))
    }
}

impl<F, R> IntoWarpService for FilteredService<F>
where
    F: Filter<Extract=R> + Send + Sync + 'static,
    R: Reply,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        self
    }
}

impl<T, R> IntoWarpService for T
where
    T: Filter<Extract=R> + Send + Sync + 'static,
    R: Reply,
{
    type Service = FilteredService<T>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService {
            filter: self,
        }
    }
}

