use ::{Filter, Request};
use ::filter::Either;
use ::reply::{Reply};
use ::route::Route;
use ::server::{WarpService};

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

    fn call(&self,  mut req: Request) -> Self::Reply {
        self.filter
            .filter(Route::new(&mut req))
            .and_then(|(route, reply)| {
                if !route.has_more_segments() {
                    Some(Either::A(reply))
                } else {
                    trace!("unmatched segments remain in route");
                    None
                }
            })
            .unwrap_or_else(|| {
                Either::B(self.not_found.call(req))
            })
    }
}

