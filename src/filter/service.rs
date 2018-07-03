use ::{Filter, Request};
use ::reply::{Reply};
use ::route::{self, Route};
use ::server::{IntoWarpService, WarpService};

#[derive(Copy, Clone, Debug)]
pub struct FilteredService<F> {
    filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    F::Future: Reply,
{
    type Reply = F::Future;

    #[inline]
    fn call(&self, req: Request) -> Self::Reply {
        debug_assert!(!route::is_set(), "nested FilteredService::calls");

        //let r = Route::new(req);
        //route::set(&r, || {
            self.filter.filter()
        //})
    }
}

impl<F> IntoWarpService for FilteredService<F>
where
    F: Filter + Send + Sync + 'static,
    F::Future: Reply,
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
    F::Future: Reply,
{
    type Service = FilteredService<F>;

    #[inline]
    fn into_warp_service(self) -> Self::Service {
        FilteredService {
            filter: self,
        }
    }
}

