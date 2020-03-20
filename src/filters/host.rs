//! Host filter
//!
use std::str::FromStr;
use crate::filter::{Filter, filter_fn_one, One};
use crate::reject::{self, Rejection};
use futures::future;
use http::uri::Authority;

/// Represents the hostname, returned by the `host()` filter.
#[derive(Debug)]
pub struct Host(Authority);

/// Try to get the host of the route
pub fn host() -> impl Filter<Extract = One<Option<Host>>, Error = Rejection> + Copy {
    let name = "Host";

    filter_fn_one(move |route| {
        future::ok(
            route.uri()
                .authority()
                .and_then(|authority| Some(Host(authority.clone())))
                .ok_or_else(|| {
                    route.headers()
                        .get(name)
                        .ok_or_else(|| reject::missing_header(name))
                        .and_then(|value| value.to_str().map_err(|_| reject::invalid_header(name)))
                        .and_then(|value| Authority::from_str(value).map_err(|_| reject::invalid_header(name)))
                        .and_then(|authority| Ok(Host(authority)))
                })
                .ok()
        )
    })
}
