//! CORS Filters

use ::filter::{Filter, WrapSealed};
use ::reject::Reject;
use ::reply::{Reply, Response};
use ::route;

use self::internal::WithCors;

/// TODO: doc
#[derive(Clone, Copy, Debug)]
pub struct Cors<F> {
   func: F,
}

/// TODO: doc
pub fn cors() -> Cors<impl Fn(Response) -> Response + Copy> {
    let func = |resp: Response| {
        route::with(|_route| {
            // If `route.headers().get("origin")` is presented,
            // add CORS headers to the response
            resp
        })
    };
    Cors {
        func,
    }
}

impl<FN, F> WrapSealed<F> for Cors<FN>
where
    FN: Fn(Response) -> Response + Clone + Send,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: Reject,
{
    type Wrapped = WithCors<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithCors {
            filter,
            cors: self.clone(),
        }
    }
}

mod internal {

    use futures::{Async, Future, Poll};

    use ::filter::{FilterBase, Filter};
    use ::reject::Reject;
    use ::reply::{Reply, ReplySealed, Response};
    use super::Cors;

    #[allow(missing_debug_implementations)]
    pub struct Corsed(pub(super) Response);

    impl ReplySealed for Corsed {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithCors<FN, F> {
        pub(super) filter: F,
        pub(super) cors: Cors<FN>,
    }

    impl<FN, F> FilterBase for WithCors<FN, F>
    where
        FN: Fn(Response) -> Response + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: Reject,
    {
        type Extract = (Corsed,);
        type Error = F::Error;
        type Future = WithCorsFuture<FN, F::Future>;

        fn filter(&self) -> Self::Future {
            WithCorsFuture {
                cors: self.cors.clone(),
                future: self.filter.filter(),
            }
        }

    }

    #[allow(missing_debug_implementations)]
    pub struct WithCorsFuture<FN, F> {
        cors: Cors<FN>,
        future: F
    }

    impl<FN, F> Future for WithCorsFuture<FN, F>
    where
        FN: Fn(Response) -> Response,
        F: Future,
        F::Item: Reply,
        F::Error: Reject,
    {
        type Item = (Corsed,);
        type Error = F::Error;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let result = match self.future.poll() {
                Ok(Async::Ready(reply)) => {
                    let resp = (self.cors.func)(reply.into_response());
                    Ok(Async::Ready((Corsed(resp),)))
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(reject) => {
                    Err(reject)
                },
            };

            result
        }
    }

}