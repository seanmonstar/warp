//! dox?
use std::str::FromStr;

use ::filter::{Cons, Filter, filter_fn, filter_fn_cons};
use ::route;


/// Create an exact match path `Filter`.
///
/// This will try to match exactly to the current request path segment.
///
/// # Note
///
/// Exact path filters cannot be empty, or contain slashes.
pub fn path(p: &'static str) -> impl Filter<Extract=(), Error=::Error> + Copy {
    assert!(!p.is_empty(), "exact path segments should not be empty");
    assert!(!p.contains('/'), "exact path segments should not contain a slash: {:?}", p);
    filter_fn(move || {
        route::with(move |route| {
            route.filter_segment(|seg| {
                trace!("({:?})?: {:?}", p, seg);
                if seg == p {
                    Some(())
                } else {
                    None
                }
            })
            .map(Ok)
            .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

/// Matches the end of a route.
pub fn index() -> impl Filter<Extract=(), Error=::Error> + Copy {
    filter_fn(move || {
        route::with(|route| {
            route.filter_segment(|seg| {
                if seg.is_empty() {
                    Some(())
                } else {
                    None
                }
            })
            .map(Ok)
            .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

/// Create an `Extract` path filter.
///
/// An `Extract` will try to parse a value from the current request path
/// segment, and if successful, the value is returned as the `Filter`'s
/// "extracted" value.
pub fn param<T: FromStr + Send>() -> impl Filter<Extract=Cons<T>, Error=::Error> + Copy {
    filter_fn_cons(move || {
        route::with(|route| {
            route.filter_segment(|seg| {
                trace!("param?: {:?}", seg);
                T::from_str(seg).ok()
            })
            .map(Ok)
            .unwrap_or_else(|| Err(::Error(())))
        })
    })
}

#[macro_export]
macro_rules! path {
    (@p $first:tt / $($tail:tt)*) => ({
        let __p = path!(@p $first);
        $(
        let __p = $crate::Filter::and(__p, path!(@p $tail));
        )*
        __p
    });
    (@p $param:ty) => (
        $crate::path::param::<$param>()
    );
    (@p $s:expr) => (
        $crate::path($s)
    );
    ($($pieces:tt)*) => (
        path!(@p $($pieces)*)
    );
}

