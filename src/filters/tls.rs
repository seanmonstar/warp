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
/// use warp::tls::Certificate;
///
/// let route = warp::tls::peer_certificates()
///     .map(|certs: Option<Vec<Certificate>>| {
///         println!("peer certificates = {:?}", certs);
///     });
/// ```
pub fn peer_certificates() -> impl Filter<Extract = (Option<Vec<Certificate>>,), Error = Infallible> + Copy {
    filter_fn_one(|route| {
        let certs = route.tls_peer_certificates()
            .map(|v|
                v.into_iter()
                .map(|c| Certificate(c))
                .collect()
                );

        futures::future::ok(certs)
    })
}

/// This type contains a single certificate by value.
#[derive(Debug)]
pub struct Certificate(Vec<u8>);

impl AsRef<[u8]> for Certificate {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Into<Vec<u8>> for Certificate {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

