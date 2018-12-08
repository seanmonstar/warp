//! Logger Filters

use std::marker::PhantomData;
use std::time::Instant;

use http::StatusCode;

use ::filter::{Filter, WrapSealed};
use ::reject::Reject;
use ::reply::Reply;
use ::route;

use self::internal::{WithLog};

/// Create a wrapping filter with the specified `name` as the `target`.
///
/// This uses the default access logging format, and log records produced
/// will have their `target` set to `name`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // If using something like `pretty_env_logger`,
/// // view logs by setting `RUST_LOG=example::api`.
/// let log = warp::log("example::api");
/// let route = warp::any()
///     .map(warp::reply)
///     .with(log);
/// ```
pub fn log(name: &'static str) -> Log<impl Fn(Info) + Copy> {
    let func = move |info: Info| {
        route::with(|route| {
            // TODO:
            // - remote_addr
            // - response content length
            // - date
            info!(
                target: name,
                "\"{} {} {:?}\" {} {:?}",
                route.method(),
                route.full_path(),
                route.version(),
                info.status.as_u16(),
                info.start.elapsed(),
            );
        });
    };
    Log {
        func,
    }
}

// TODO:
// pub fn custom(impl Fn(Info)) -> Log

/// Decorates a [`Filter`](::Filter) to log requests and responses.
#[derive(Clone, Copy, Debug)]
pub struct Log<F> {
    func: F,
}

/// Information about the request/response that can be used to prepare log lines.
#[allow(missing_debug_implementations)]
pub struct Info<'a> {
    start: Instant,
    status: StatusCode,
    // This struct will eventually hold a `&'a Route` and `&'a Response`,
    // so use a marker so there can be a lifetime in the struct definition.
    _marker: PhantomData<&'a ()>,
}

impl<FN, F> WrapSealed<F> for Log<FN>
where
    FN: Fn(Info) + Clone + Send,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: Reject,
{
    type Wrapped = WithLog<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithLog {
            filter,
            log: self.clone(),
        }
    }
}

mod internal {
    use std::marker::PhantomData;
    use std::time::Instant;

    use futures::{Async, Future, Poll};

    use ::filter::{FilterBase, Filter};
    use ::reject::Reject;
    use ::reply::{Reply, ReplySealed, Response};
    use super::{Info, Log};

    #[allow(missing_debug_implementations)]
    pub struct Logged(pub(super) Response);

    impl ReplySealed for Logged {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithLog<FN, F> {
        pub(super) filter: F,
        pub(super) log: Log<FN>,
    }

    impl<FN, F> FilterBase for WithLog<FN, F>
    where
        FN: Fn(Info) + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: Reject,
    {
        type Extract = (Logged,);
        type Error = F::Error;
        type Future = WithLogFuture<FN, F::Future>;

        fn filter(&self) -> Self::Future {
            let started = Instant::now();
            WithLogFuture {
                log: self.log.clone(),
                future: self.filter.filter(),
                started,
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct WithLogFuture<FN, F> {
        log: Log<FN>,
        future: F,
        started: Instant,
    }

    impl<FN, F> Future for WithLogFuture<FN, F>
    where
        FN: Fn(Info),
        F: Future,
        F::Item: Reply,
        F::Error: Reject,
    {
        type Item = (Logged,);
        type Error = F::Error;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let (result, status) = match self.future.poll() {
                Ok(Async::Ready(reply)) => {
                    let resp = reply.into_response();
                    let status = resp.status();
                    (Ok(Async::Ready((Logged(resp),))), status)
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(reject) => {
                    let status = reject.status();
                    (Err(reject), status)
                },
            };

            (self.log.func)(Info {
                start: self.started,
                status,
                _marker: PhantomData,
            });

            result
        }
    }
}

