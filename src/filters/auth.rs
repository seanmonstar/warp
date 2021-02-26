//! HTTP Basic Authentication Filters

use futures::future;
use headers::{authorization::Basic, Authorization};
use std::error::Error as StdError;

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
        .or_else(move |_| future::err(crate::reject::unauthorized("Basic", realm)))
}

/// Unauthorized request header
#[derive(Debug)]
pub(crate) struct UnauthorizedChallenge {
    pub realm: &'static str,
    pub scheme: &'static str,
    //content_type: &'static str, // TODO:  make it json compatible??
}

impl UnauthorizedChallenge {
    /*
    /// Realm name of the Authoriziation
    pub fn realm(&self) -> &str {
        self.realm
    }

    /// Scheme of the Authoriziation (e.g. Basic, Bearer,...)
    pub fn scheme(&self) -> &str {
        self.scheme
    }
    */
}

impl ::std::fmt::Display for UnauthorizedChallenge {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Unauthorized request")
    }
}

impl StdError for UnauthorizedChallenge {}
