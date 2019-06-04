//! Query Filters

use std::error::Error as StdError;

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
        let query_string = route.query().unwrap_or_else(|| {
            debug!("route was called without a query string, defaulting to empty");
            ""
        });

        serde_urlencoded::from_str(query_string).map_err(|e| {
            debug!("failed to decode query string '{}': {:?}", query_string, e);
            reject::known(InvalidQuery)
        })
    })
}

/// Creates a `Filter` that returns the raw query string as type String.
pub fn raw() -> impl Filter<Extract = One<String>, Error = Rejection> + Copy {
    filter_fn_one(|route| {
        route
            .query()
            .map(|q| q.to_owned())
            .map(Ok)
            .unwrap_or_else(|| Err(reject::known(InvalidQuery)))
    })
}

#[derive(Debug)]
pub(crate) struct InvalidQuery;

impl ::std::fmt::Display for InvalidQuery {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.write_str("Invalid query string")
    }
}

impl StdError for InvalidQuery {
    fn description(&self) -> &str {
        "Invalid query string"
    }
}
