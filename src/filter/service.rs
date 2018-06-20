use ::{Filter, Request};
use ::filter::Either;
use ::reply::{NOT_FOUND, NotFound, Reply};
use ::route::{self, Route};
use ::server::{IntoWarpService, WarpService};

#[derive(Debug)]
pub struct FilteredService<F, N> {
    pub(super) filter: F,
    pub(super) not_found: N,
}

impl<F, N> WarpService for FilteredService<F, N>
where
    F: Filter,
    F::Extract: Reply,
    N: WarpService,
{
    type Reply = Either<F::Extract, N::Reply>;

    fn call(&self,  req: Request) -> Self::Reply {
        debug_assert!(!route::is_set(), "nested FilteredService::calls");

        let r = Route::new(req);
        route::set(&r, || {
            self.filter.filter()
        })
            .and_then(|reply| {
                if !r.has_more_segments() {
                    Some(Either::A(reply))
                } else {
                    trace!("unmatched segments remain in route");
                    None
                }
            })
            .unwrap_or_else(|| {
                Either::B(self.not_found.call(r.into_req()))
            })
    }
}

impl<F, N> IntoWarpService for FilteredService<F, N>
where
    F: Filter + Send + Sync + 'static,
    F::Extract: Reply,
    N: WarpService + Send + Sync + 'static,
{
    type Service = FilteredService<F, N>;

    fn into_warp_service(self) -> Self::Service {
        self
    }
}

impl<T> IntoWarpService for T
where
    T: Filter + Send + Sync + 'static,
    T::Extract: Reply,
{
    type Service = FilteredService<T, NotFound>;

    fn into_warp_service(self) -> Self::Service {
        self.service_with_not_found(NOT_FOUND)
    }
}
