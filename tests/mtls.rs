#![deny(warnings)]
#![cfg(feature = "tls")]

use rustls_pki_types::CertificateDer;

#[tokio::test]
async fn peer_certificates_missing() {
    let extract_peer_certs = warp::mtls::peer_certificates();

    let req = warp::test::request();
    let resp = req.filter(&extract_peer_certs).await.unwrap();
    assert!(resp.is_none())
}

#[tokio::test]
async fn peer_certificates_present() {
    let extract_peer_certs = warp::mtls::peer_certificates();

    let cert = CertificateDer::<'_>::from_slice(b"TEST CERT");

    let req = warp::test::request().peer_certificates([cert.clone()]);
    let resp = req.filter(&extract_peer_certs).await.unwrap();
    assert_eq!(resp.unwrap(), &[cert],)
}
