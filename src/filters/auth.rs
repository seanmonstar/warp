//! HTTP Basic Authentication Filters

use futures::future;
use headers::{authorization::Basic, Authorization};

use crate::filter::Filter;
use crate::reject::Rejection;

use super::header;

/// Creates a `Filter` to the HTTP Basic Authentication header. If none was sent by the client, this filter will reject
/// any request with a 401 Unauthorized.
///
/// # Example
///
/// ```
/// use std::net::SocketAddr;
/// use warp::Filter;
/// use headers::authorization::Basic;
///
/// let route = warp::auth::basic("Realm name")
///     .map(|auth_header: Basic| {
///         println!("authorization header = {:?}", auth_header);
///     });
/// ```
pub fn basic(realm: &'static str) -> impl Filter<Extract = (Basic,), Error = Rejection> + Copy {
    header::header2()
        .map(|auth: Authorization<Basic>| auth.0)
        .or_else(move |_| future::err(crate::reject::unauthorized(realm)))
}
