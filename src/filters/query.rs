//! dox?

use serde::de::DeserializeOwned;
use serde_urlencoded;

use ::filter::{Cons, Filter, filter_fn_cons};
use ::route;

/// Creates a `Filter` that decodes query parameters to the type `T`.
///
/// If cannot decode into a `T`, the request is rejected.
pub fn query<T: DeserializeOwned + Send>() -> impl Filter<Extract=Cons<T>> + Copy {
    filter_fn_cons(|| {
        route::with(|route| {
            route
                .query()
                .and_then(|q| {
                    serde_urlencoded::from_str(q)
                        .ok()
                })
                .map(Ok)
                .unwrap_or_else(|| Err(::Error(())))
        })
    })
}
