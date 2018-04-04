
use http::{Request, Response};
use hyper::Body;

use ::{Filter};
use ::reply::{Reply, WarpBody};
use ::server::{WarpService};

pub struct FilteredService<F> {
    pub(super) filter: F,
}

impl<F> WarpService for FilteredService<F>
where
    F: Filter,
    F::Extract: Reply,
{
    type Reply = Response<WarpBody>;

    fn call(&self, mut req: Request<WarpBody>) -> Self::Reply {
        self.filter
            .filter(&mut req)
            .map(Reply::into_response)
            .unwrap_or_else(|| {
                Response::builder()
                    .status(404)
                    .header("content-length", "0")
                    .body(WarpBody(Body::empty()))
                    .unwrap()
            })
    }
}

