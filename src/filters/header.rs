//! Header Filters
//!
//! These filters are used to interact with the Request HTTP headers. Some
//! of them, like `exact` and `exact_ignore_case`, are just predicates,
//! they don't extract any values. The `header` filter allows parsing
//! a type from any header.
use std::str::FromStr;

use headers::{Header, HeaderMapExt};
use http::HeaderMap;

use ::never::Never;
use ::filter::{Filter, filter_fn, filter_fn_one, One};
use ::reject::{self, Rejection};

/// Create a `Filter` that tries to parse the specified header.
///
/// This `Filter` will look for a header with supplied name, and try to
/// parse to a `T`, otherwise rejects the request.
///
/// # Example
///
/// ```
/// use std::net::SocketAddr;
///
/// // Parse `content-length: 100` as a `u64`
/// let content_length = warp::header::<u64>("content-length");
///
/// // Parse `host: 127.0.0.1:8080` as a `SocketAddr
/// let local_host = warp::header::<SocketAddr>("host");
///
/// // Parse `foo: bar` into a `String`
/// let foo = warp::header::<String>("foo");
/// ```
pub fn header<T: FromStr + Send>(name: &'static str) -> impl Filter<Extract=One<T>, Error=Rejection> + Copy {
    filter_fn_one(move |route| {
        trace!("header({:?})", name);
        route.headers()
            .get(name)
            .and_then(|val| {
                val.to_str().ok()
            })
            .and_then(|s| {
                T::from_str(s)
                    .ok()
            })
            .map(Ok)
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}

pub(crate) fn header2<T: Header + Send>() -> impl Filter<Extract=One<T>, Error=Rejection> + Copy {
    filter_fn_one(move |route| {
        trace!("header2({:?})", T::NAME);
        route.headers()
            .typed_get()
            .ok_or_else(|| reject::bad_request())
    })
}

/* TODO
pub fn exact2<T>(header: T) -> impl FilterClone<Extract=(), Error=Rejection>
where
    T: Header + PartialEq + Clone + Send,
{
    filter_fn(move |route| {
        trace!("exact2({:?})", T::NAME);
        route.headers()
            .typed_get::<T>()
            .and_then(|val| if val == header {
                Some(())
            } else {
                None
            })
            .ok_or_else(|| reject::bad_request())
    })
}
*/

/// Create a `Filter` that requires a header to match the value exactly.
///
/// This `Filter` will look for a header with supplied name and the exact
/// value, otherwise rejects the request.
///
/// # Example
///
/// ```
/// // Require `dnt: 1` header to be set.
/// let must_dnt = warp::header::exact("dnt", "1");
/// ```
pub fn exact(name: &'static str, value: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Copy {
    filter_fn(move |route| {
        trace!("exact({:?}, {:?})", name, value);
        route.headers()
            .get(name)
            .map(|val| {
                if val == value {
                    Ok(())
                } else {
                    // TODO: exact header error kind?
                    Err(reject::bad_request())
                }
            })
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}

/// Create a `Filter` that requires a header to match the value exactly.
///
/// This `Filter` will look for a header with supplied name and the exact
/// value, ignoring ASCII case, otherwise rejects the request.
///
/// # Example
///
/// ```
/// // Require `connection: keep-alive` header to be set.
/// let keep_alive = warp::header::exact("connection", "keep-alive");
/// ```
pub fn exact_ignore_case(name: &'static str, value: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Copy {
    filter_fn(move |route| {
        trace!("exact_ignore_case({:?}, {:?})", name, value);
        route.headers()
            .get(name)
            .map(|val| {
                trace!("    -> {:?}", val);
                if val.as_bytes().eq_ignore_ascii_case(value.as_bytes()) {
                    Ok(())
                } else {
                    // TODO: exact header error kind
                    Err(reject::bad_request())
                }
            })
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}

/// Create a `Filter` that returns a clone of the request's `HeaderMap`.
///
/// # Example
///
/// ```
/// use warp::{Filter, http::HeaderMap};
///
/// let headers = warp::header::headers_cloned()
///     .map(|headers: HeaderMap| {
///         format!("header count: {}", headers.len())
///     });
/// ```
pub fn headers_cloned() -> impl Filter<Extract=One<HeaderMap>, Error=Never> + Copy {
    filter_fn_one(|route| {
        Ok(route.headers().clone())
    })
}

pub(crate) fn optional<T>()
    -> impl Filter<Extract=One<Option<T>>, Error=Never> + Copy
where
    T: Header + Send,
{
    filter_fn_one(move |route| {
        Ok(route.headers().typed_get())
    })
}

