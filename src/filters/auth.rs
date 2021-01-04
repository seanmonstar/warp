//! HTTP Authentication Filters

pub use headers::{
    authorization::Basic,
    Authorization
};
use futures::future;

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
/// use headers::Authorization;
///
/// let route = warp::addr::basic("Realm name")
///     .map(|auth_header: Authorization<Basic>| {
///         println!("authorization header = {:?}", auth_header);
///     });
/// ```
pub fn basic(_realm: &str) -> impl Filter<Extract = (Authorization<Basic>,), Error = Rejection> {
    header::header2()
        //.and_then(move |auth_header: Authorization<Basic>| future::ok(auth_header))
        .or_else(|_| {
            // TODO: reply with header: `WWW-Authenticate: Basic realm="..."`
            future::err(crate::reject::unauthorized())
        })
}
