//! Query Filters

use serde::de::DeserializeOwned;
use serde_urlencoded;

use filter::{filter_fn_one, Filter, One};
use reject::{self, Rejection};

/// Creates a `Filter` that decodes query parameters to the type `T`.
///
/// If cannot decode into a `T`, the request is rejected with a `400 Bad Request`.
pub fn query<T: DeserializeOwned + Send>() -> impl Filter<Extract = One<T>, Error = Rejection> + Copy
{
    filter_fn_one(|route| {
        route
            .query()
            .and_then(|q| serde_urlencoded::from_str(q).ok())
            .map(Ok)
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}

/// Creates a `Filter` that returns the raw query string as type String.
pub fn raw() -> impl Filter<Extract = One<String>, Error = Rejection> + Copy {
    filter_fn_one(|route| {
        route
            .query()
            .map(|q| q.to_owned())
            .map(Ok)
            .unwrap_or_else(|| Err(reject::bad_request()))
    })
}
