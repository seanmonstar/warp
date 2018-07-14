//! Header Filters
use std::str::FromStr;

use http::header::HeaderValue;

use ::never::Never;
use ::filter::{Filter, filter_fn, filter_fn_one, One};
use ::reject::{self, Rejection};

pub(crate) fn value<F, U>(name: &'static str, func: F)
    -> impl Filter<Extract=One<U>, Error=Rejection> + Copy
where
    F: Fn(&HeaderValue) -> Option<U> + Copy,
    U: Send,
{
    filter_fn_one(move |route| {
        route.headers()
            .get(name)
            .and_then(func)
            .map(Ok)
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}

pub(crate) fn optional_value<F, U>(name: &'static str, func: F)
    -> impl Filter<Extract=One<Option<U>>, Error=Never> + Copy
where
    F: Fn(&HeaderValue) -> Option<U> + Copy,
    U: Send,
{
    filter_fn_one(move |route| {
        Ok::<_, Never>(route.headers()
            .get(name)
            .and_then(func))
    })
}

/// Return an extract `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name,
/// and try to parse to a `T`, otherwise rejects the request.
pub fn header<T: FromStr + Send>(name: &'static str) -> impl Filter<Extract=One<T>, Error=Rejection> + Copy {
    filter_fn_one(move |route| {
        trace!("header::Extract({:?})", name);
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

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, otherwise rejects the request.
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

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, ignoring ASCII case, otherwise rejects the request.
pub fn exact_ignore_case(name: &'static str, value: &'static str) -> impl Filter<Extract=(), Error=Rejection> + Copy {
    filter_fn(move |route| {
        trace!("exact_ignore_case({:?}, {:?})", name, value);
        route.headers()
            .get(name)
            .map(|val| {
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

