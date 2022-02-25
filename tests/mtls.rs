#![deny(warnings)]
#![cfg(feature = "tls")]

use tokio_rustls::rustls::Certificate;

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

    let cert = Certificate(b"TEST CERT".to_vec());
    let req = warp::test::request().peer_certificates([cert.clone()]);
    let resp = req.filter(&extract_peer_certs).await.unwrap();
    assert_eq!(
        resp.unwrap().as_ref(),
        &[cert],
    )
}
