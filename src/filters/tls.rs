//! Tls filters

use std::convert::Infallible;

use crate::filter::{filter_fn_one, Filter};

/// Creates a `Filter` to get the remote tls certificate chain of the connection.
///
/// If the underlying connection doesn't have certificates, this will yield
/// `None`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::::tls::peer_certificates()
///     .map(|certs: Option<Vec<Vec<u8>>>| {
///         println!("peer certificates = {:?}", certs);
///     });
/// ```
pub fn peer_certificates() -> impl Filter<Extract = (Option<Vec<Vec<u8>>>,), Error = Infallible> + Copy {
    filter_fn_one(|route| futures::future::ok(route.tls_peer_certificates()))
}
