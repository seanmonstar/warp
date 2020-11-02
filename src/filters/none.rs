//! A filter that matches no route.
use futures::future;

use crate::filter::{filter_fn, Filter};
use crate::reject::{self, Rejection};

/// A filter that matches no route.
///
/// This can be useful to help with styling.
///
/// # Example
///
/// ```
/// let routes = warp::none()
///     .or(route_1)
///     .or(route_2)
///     .or(route_3)
/// ```
///
/// looks nicer than the following because the routes are lined up
/// ```
/// let routes = route_1
///     .or(route_2)
///     .or(route_3)
/// ```
pub fn none() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    // always reject with not found
    filter_fn(|_route| future::ready(Err(reject::not_found())))
}
