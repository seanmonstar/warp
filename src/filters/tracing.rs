//! [`tracing`] filters.
//!
//! [`tracing`] tracing is a framework for instrumenting Rust programs to
//! collect scoped, structured, and async-aware diagnostics. This module
//! provides a set of filters for instrumenting Warp applications with `tracing`
//! spans. [Spans] can be used to associate individual events  with a request,
//! and track contexts through the application.
//!
//! [`tracing`]: https://crates.io/tracing
/// [Spans]: https://docs.rs/tracing/latest/tracing/#spans
use tracing::Span;

use std::fmt;
use std::net::SocketAddr;

use http::{self, header};

use crate::filter::{Filter, WrapSealed};
use crate::reject::IsReject;
use crate::reply::Reply;
use crate::route::Route;

use self::internal::WithTrace;

/// Create a wrapping filter that instruments every request with a `tracing`
/// [`Span`] at the [`INFO`] level, containing a summary of the request.
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
///
/// [`Span`]: https://docs.rs/tracing/latest/tracing/#spans
/// [`INFO`]: https://docs.rs/tracing/0.1.16/tracing/struct.Level.html#associatedconstant.INFO
pub fn tracing() -> Trace<impl Fn(Info) -> Span + Clone> {
    use tracing::field::{display, Empty};
    let func = move |info: Info| {
        let span = tracing::info_span!(
            "request",
            remote_addr = Empty,
            method = %info.method(),
            path = %info.path(),
            version = ?info.route.version(),
            referer = Empty,
            user_agent = Empty,
        );

        // Record optional fields.
        if let Some(remote_addr) = info.remote_addr() {
            span.record("remote_addr", &display(remote_addr));
        }

        if let Some(referer) = info.referer() {
            span.record("referer", &display(referer));
        }

        if let Some(user_agent) = info.user_agent() {
            span.record("user_agent", &display(user_agent));
        }

        span
    };
    Trace { func }
}

/// Create a wrapping filter that instruments every request with a custom
/// `tracing` [`Span`] provided by a function.
///
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
///     )
/// });
/// let route = warp::any()
///     .map(warp::reply)
///     .with(tracing);
/// ```
///
/// [`Span`]: https://docs.rs/tracing/latest/tracing/#spans
pub fn custom<F>(func: F) -> Trace<F>
where
    F: Fn(Info) -> Span + Clone,
{
    Trace { func }
}

/// Create a wrapping filter that instruments every request with a `tracing`
/// [`Span`] at the [`DEBUG`] level representing a named context.
///
/// This can be used to instrument multiple routes with their own sub-spans in a
/// per-request trace.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let hello = warp::path("hello")
///     .map(warp::reply)
///     .with(warp::tracing::context("hello"));
///
/// let goodbye = warp::path("goodbye")
///     .map(warp::reply)
///     .with(warp::tracing::context("goodbye"));
///
/// let routes = hello.or(goodbye);
/// ```
///
/// [`Span`]: https://docs.rs/tracing/latest/tracing/#spans
/// [`DEBUG`]: https://docs.rs/tracing/0.1.16/tracing/struct.Level.html#associatedconstant.DEBUG
pub fn context(name: &'static str) -> Trace<impl Fn(Info<'_>) -> tracing::Span + Copy> {
    custom(move |_| {
        tracing::debug_span!(
            target: "warp",
            "context",
            "{}", name,
        )
    })
}

/// Decorates a [`Filter`](crate::Filter) to create a [`tracing`] [span] for
/// requests and responses.
///
/// [`tracing`]: https://crates.io/tracing
/// [span]: https://docs.rs/tracing/latest/tracing/#spans
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
    use futures::{future::Inspect, future::MapOk, FutureExt, TryFutureExt};

    use super::{Info, Trace};
    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::Reply;
    use crate::reply::Response;
    use crate::route;

    #[allow(missing_debug_implementations)]
    pub struct Traced(pub(super) Response);

    impl Reply for Traced {
        #[inline]
        fn into_response(self) -> Response {
            self.0
        }
    }

    #[allow(missing_debug_implementations)]
    #[derive(Clone, Copy)]
    pub struct WithTrace<FN, F> {
        pub(super) filter: F,
        pub(super) trace: Trace<FN>,
    }

    use tracing::Span;
    use tracing_futures::{Instrument, Instrumented};

    fn finished_logger<E: IsReject>(reply: &Result<(Traced,), E>) {
        match reply {
            Ok((Traced(resp),)) => {
                tracing::info!(target: "warp::filters::tracing", status = resp.status().as_u16(), "finished processing with success");
            }
            Err(e) if e.status().is_server_error() => {
                tracing::error!(
                    target: "warp::filters::tracing",
                    status = e.status().as_u16(),
                    error = ?e,
                    "unable to process request (internal error)"
                );
            }
            Err(e) if e.status().is_client_error() => {
                tracing::warn!(
                    target: "warp::filters::tracing",
                    status = e.status().as_u16(),
                    error = ?e,
                    "unable to serve request (client error)"
                );
            }
            Err(e) => {
                // Either informational or redirect
                tracing::info!(
                    target: "warp::filters::tracing",
                    status = e.status().as_u16(),
                    result = ?e,
                    "finished processing with status"
                );
            }
        }
    }

    fn convert_reply<R: Reply>(reply: R) -> (Traced,) {
        (Traced(reply.into_response()),)
    }

    impl<FN, F> FilterBase for WithTrace<FN, F>
    where
        FN: Fn(Info) -> Span + Clone + Send,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Extract = (Traced,);
        type Error = F::Error;
        type Future = Instrumented<
            Inspect<
                MapOk<F::Future, fn(F::Extract) -> Self::Extract>,
                fn(&Result<Self::Extract, F::Error>),
            >,
        >;

        fn filter(&self, _: Internal) -> Self::Future {
            let span = route::with(|route| (self.trace.func)(Info { route }));
            let _entered = span.enter();

            tracing::info!(target: "warp::filters::tracing", "processing request");
            self.filter
                .filter(Internal)
                .map_ok(convert_reply as fn(F::Extract) -> Self::Extract)
                .inspect(finished_logger as fn(&Result<Self::Extract, F::Error>))
                .instrument(span.clone())
        }
    }
}
