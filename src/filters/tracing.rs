//! Tracing Filters

use tracing::Span;

use std::fmt;
use std::net::SocketAddr;

use http::{self, header};

use crate::filter::{Filter, WrapSealed};
use crate::reject::IsReject;
use crate::reply::Reply;
use crate::route::Route;

use self::internal::WithTrace;

/// Create a wrapping filter which adds a span with request info
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any()
///     .map(warp::reply)
///     .with(warp::tracing());
/// ```
pub fn tracing() -> Trace<impl Fn(Info) -> Span + Clone> {
    let func = move |info: Info| {
        tracing::info_span!(
            "request",
            remote_addr = %OptFmt(info.route.remote_addr()),
            method = %info.method(),
            path = %info.path(),
            version = ?info.route.version(),
            // status = %info.status().as_u16(),
            referer = %OptFmt(info.referer()),
            user_agent = %OptFmt(info.user_agent()),
        )
    };
    Trace { func }
}

/// Create a wrapping filter which adds a custom span with request info
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let tracing = warp::tracing::custom(|info| {
///     // Create a span using tracing macros
///     tracing::info_span!(
///         "request",
///         method = %info.method(),
///         path = %info.path(),
///     );
/// });
/// let route = warp::any()
///     .map(warp::reply)
///     .with(tracing);
/// ```
pub fn custom<F>(func: F) -> Trace<F>
where
    F: Fn(Info) -> Span + Clone,
{
    Trace { func }
}

/// Decorates a [`Filter`](::Filter) to log requests and responses.
#[derive(Clone, Copy, Debug)]
pub struct Trace<F> {
    func: F,
}

/// Information about the request/response that can be used to prepare log lines.
#[allow(missing_debug_implementations)]
pub struct Info<'a> {
    route: &'a Route,
}

impl<FN, F> WrapSealed<F> for Trace<FN>
where
    FN: Fn(Info) -> Span + Clone + Send,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Wrapped = WithTrace<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithTrace {
            filter,
            trace: self.clone(),
        }
    }
}

impl<'a> Info<'a> {
    /// View the remote `SocketAddr` of the request.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.route.remote_addr()
    }

    /// View the `http::Method` of the request.
    pub fn method(&self) -> &http::Method {
        self.route.method()
    }

    /// View the URI path of the request.
    pub fn path(&self) -> &str {
        self.route.full_path()
    }

    /// View the `http::Version` of the request.
    pub fn version(&self) -> http::Version {
        self.route.version()
    }

    /// View the referer of the request.
    pub fn referer(&self) -> Option<&str> {
        self.route
            .headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
    }

    /// View the user agent of the request.
    pub fn user_agent(&self) -> Option<&str> {
        self.route
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
    }

    /// View the host of the request
    pub fn host(&self) -> Option<&str> {
        self.route
            .headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
    }
}

struct OptFmt<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for OptFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref t) = self.0 {
            fmt::Display::fmt(t, f)
        } else {
            f.write_str("-")
        }
    }
}

mod internal {
    use futures::{future::Inspect, FutureExt};

    use std::future::Future;

    use super::{Info, Trace};
    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::Reply;
    use crate::route;

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithTrace<FN, F> {
        pub(super) filter: F,
        pub(super) trace: Trace<FN>,
    }

    use tracing::Span;
    use tracing_futures::{Instrument, Instrumented};

    impl<FN, F> FilterBase for WithTrace<FN, F>
    where
        FN: Fn(Info) -> Span + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Extract = F::Extract;
        type Error = F::Error;
        type Future = Inspect<Instrumented<F::Future>, fn(&<F::Future as Future>::Output)>;

        fn filter(&self, _: Internal) -> Self::Future {
            let span = route::with(|route| (self.trace.func)(Info { route }));
            let _guard = span.enter();

            fn finished_logger<R: Reply, E: IsReject>(reply: &Result<R, E>) {
                if let Err(e) = reply {
                    tracing::warn!(target: "warp::filters::tracing", msg = ?e, "error during processing");
                } else {
                    tracing::info!(target: "warp::filters::tracing", "successfully finished processing");
                }
            }

            tracing::info!(target: "warp::filters::tracing", "processing request");
            self.filter
                .filter(Internal)
                .in_current_span()
                .inspect(finished_logger)
        }
    }
}
