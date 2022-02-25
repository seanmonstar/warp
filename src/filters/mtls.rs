//! Mutual (client) TLS filters.

use std::convert::Infallible;

use tokio_rustls::rustls::Certificate;

use crate::{
    filter::{filter_fn_one, Filter},
    route::Route,
};

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
    filter_fn_one(|route| futures_util::future::ok(Certificates::from_route(route)))
}

/// Certificates is a iterable container of Certificates.
#[derive(Debug)]
pub struct Certificates(Vec<Certificate>);

impl Certificates {
    fn from_route(route: &Route) -> Option<Certificates> {
        route
            .peer_certificates()
            .read()
            .unwrap()
            .as_ref()
            .map(|certs| Self(certs.to_vec()))
    }
}

impl AsRef<[Certificate]> for Certificates {
    fn as_ref(&self) -> &[Certificate] {
        self.0.as_ref()
    }
}

impl<'a> IntoIterator for &'a Certificates {
    type Item = &'a Certificate;
    type IntoIter = std::slice::Iter<'a, Certificate>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
