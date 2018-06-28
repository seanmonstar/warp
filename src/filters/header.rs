//! dox?
use std::str::FromStr;

use ::filter::{Cons, FilterAnd, filter_fn, filter_fn_cons};
use ::route;

/// Return an extract `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name,
/// and try to parse to a `T`, otherwise rejects the request.
pub fn header<T: FromStr>(name: &'static str) -> impl FilterAnd<Extract=Cons<T>> {
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
        })
    })
}

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, otherwise rejects the request.
pub fn exact(name: &'static str, value: &'static str) -> impl FilterAnd<Extract=()> {
    filter_fn(move || {
        trace!("header::Exact({:?}, {:?})", name, value);
        route::with(|route| {
            route.headers()
                .get(name)
                .and_then(|val| {
                    if val == value {
                        Some(())
                    } else {
                        None
                    }
                })
        })
    })
}

