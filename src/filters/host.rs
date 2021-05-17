//! Host ("authority") filter
//!
use crate::filter::{filter_fn_one, Filter, One};
use crate::reject::{self, Rejection};
use futures::future;
pub use http::uri::Authority;
use std::str::FromStr;

/// Creates a `Filter` that requires a specific authority (target server's
/// host and port) in the request.
///
/// Authority is specified either in the `Host` header or in the target URI.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let multihost =
///     warp::host::exact("foo.com").map(|| "you've reached foo.com")
///     .or(warp::host::exact("bar.com").map(|| "you've reached bar.com"));
/// ```
pub fn exact(expected: &str) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    let expected = Authority::from_str(expected).expect("invalid host/authority");
    optional()
        .and_then(move |option: Option<Authority>| match option {
            Some(authority) if authority == expected => future::ok(()),
            _ => future::err(reject::not_found()),
        })
        .untuple_one()
}

/// Creates a `Filter` that looks for an authority (target server's host
/// and port) in the request.
///
/// Authority is specified either in the `Host` header or in the target URI.
///
/// If found, extracts the `Authority`, otherwise continues the request,
/// extracting `None`.
///
/// Rejects with `400 Bad Request` if the `Host` header is malformed or if there
/// is a mismatch between the `Host` header and the target URI.
///
/// # Example
///
/// ```
/// use warp::{Filter, host::Authority};
///
/// let host = warp::host::optional()
///     .map(|authority: Option<Authority>| {
///         if let Some(a) = authority {
///             format!("{} is currently not at home", a.host())
///         } else {
///             "please state who you're trying to reach".to_owned()
///         }
///     });
/// ```
pub fn optional() -> impl Filter<Extract = One<Option<Authority>>, Error = Rejection> + Copy {
    filter_fn_one(move |route| {
        future::ready(route.host().map_err(|_| reject::invalid_header("host")))
    })
}
