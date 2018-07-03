//! dox?
use std::str::FromStr;

use http::header::HeaderValue;

use ::never::Never;
use ::filter::{Cons, Filter, filter_fn, filter_fn_cons};
use ::route;

pub(crate) fn value<F, U>(name: &'static str, func: F)
    -> impl Filter<Extract=Cons<U>, Error=::Error> + Copy
where
    F: Fn(&HeaderValue) -> Option<U> + Copy,
    U: Send,
{
    filter_fn_cons(move || {
        route::with(|route| {
            route.headers()
                .get(name)
                .and_then(func)
                .map(Ok)
                .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

pub(crate) fn optional_value<F, U>(name: &'static str, func: F)
    -> impl Filter<Extract=Cons<Option<U>>> + Copy
where
    F: Fn(&HeaderValue) -> Option<U> + Copy,
    U: Send,
{
    filter_fn_cons(move || {
        route::with(|route| {
            Ok::<_, Never>(route.headers()
                .get(name)
                .and_then(func))
        })
    })
}

/// Return an extract `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name,
/// and try to parse to a `T`, otherwise rejects the request.
pub fn header<T: FromStr + Send>(name: &'static str) -> impl Filter<Extract=Cons<T>, Error=::Error> + Copy {
    filter_fn_cons(move || {
        trace!("header::Extract({:?})", name);
        route::with(|route| {
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
                .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, otherwise rejects the request.
pub fn exact(name: &'static str, value: &'static str) -> impl Filter<Extract=(), Error=::Error> + Copy {
    filter_fn(move || {
        trace!("exact({:?}, {:?})", name, value);
        route::with(|route| {
            route.headers()
                .get(name)
                .map(|val| {
                    if val == value {
                        Ok(())
                    } else {
                        // TODO: exact header error kind
                        Err(::Error(()))
                    }
                })
                // TODO: missing header error kind
                .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, ignoring ASCII case, otherwise rejects the request.
pub fn exact_ignore_case(name: &'static str, value: &'static str) -> impl Filter<Extract=(), Error=::Error> + Copy {
    filter_fn(move || {
        trace!("exact_ignore_case({:?}, {:?})", name, value);
        route::with(|route| {
            route.headers()
                .get(name)
                .map(|val| {
                    if val.as_bytes().eq_ignore_ascii_case(value.as_bytes()) {
                        Ok(())
                    } else {
                        // TODO: exact header error kind
                        Err(::Error(()))
                    }
                })
                // TODO: missing header error kind
                .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

