#![deny(warnings)]

// Don't copy this `cfg`, it's only needed because this file is within
// the warp repository.
#[cfg(feature = "tls")]
#[tokio::main]
async fn main() {
    use warp::Filter;

    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");

    let mut tls_config = warp::tls::TlsConfigBuilder::new();
    tls_config
        .set_cert_path("examples/tls/cert.pem").unwrap()
        .set_key_path("examples/tls/key.rsa").unwrap();

    warp::serve(routes)
        .tls(&mut tls_config)
        .run(([127, 0, 0, 1], 3030)).await;
}

#[cfg(not(feature = "tls"))]
fn main() {
    eprintln!("Requires the `tls` feature.");
}
