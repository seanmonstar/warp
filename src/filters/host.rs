//! Host filter
//!
use futures::future;
use crate::reject::{self, Rejection};
use crate::filter::{Filter, filter_fn, one};

/// Get the host of the route
pub fn host() -> impl Filter<Extract = (Option<String>,), Error = Rejection> + Copy {
    let name = "Host";

    filter_fn(move |route| future::ok(one(
        route.uri()
            .authority()
            .and_then(|authority| Some(authority.host()))
            .and_then(|host| Some(host.to_string()))
            .or_else(|| route.headers()
                .get(name)
                .ok_or_else(|| reject::missing_header(name))
                .and_then(|value| value.to_str().map_err(|_| reject::invalid_header(name)))
                .and_then(|s| Ok(String::from(s)))
                .ok()
            )
        ))
    )
}
