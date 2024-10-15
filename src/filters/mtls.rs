//! Mutual (client) TLS filters.

use std::convert::Infallible;

use rustls_pki_types::CertificateDer;

use crate::{
    filter::{filter_fn_one, Filter},
    route::Route,
};

/// Certificates is a iterable container of Certificates.
pub type Certificates = Vec<CertificateDer<'static>>;

/// Creates a `Filter` to get the peer certificates for the TLS connection.
///
/// If the underlying transport doesn't have peer certificates, this will yield
/// `None`.
///
/// # Example
///
/// ```
/// use warp::mtls::Certificates;
/// use warp::Filter;
///
/// let route = warp::mtls::peer_certificates()
///     .map(|certs: Option<Certificates>| {
///         println!("peer certificates = {:?}", certs.as_ref());
///     });
/// ```
pub fn peer_certificates(
) -> impl Filter<Extract = (Option<Certificates>,), Error = Infallible> + Copy {
    filter_fn_one(|route| futures_util::future::ok(from_route(route)))
}

/// Testing
pub fn peer_certs_into_owned(certs: &Vec<CertificateDer<'_>>) -> Vec<CertificateDer<'static>> {
    certs
        .to_vec()
        .iter()
        .map(|cert| cert.clone().into_owned())
        .collect()
}

fn from_route(route: &Route) -> Option<Certificates> {
    route
        .peer_certificates()
        .read()
        .unwrap()
        .as_ref()
        .map(peer_certs_into_owned)
}
