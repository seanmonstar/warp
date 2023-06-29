#![deny(warnings)]

// Don't copy this `cfg`, it's only needed because this file is within
// the warp repository.
// Instead, specify the "tls" feature in your warp dependency declaration.
#[cfg(any(feature = "tls-openssl", feature = "tls"))]
#[tokio::main]
async fn main() {
    use warp::Filter;
    pretty_env_logger::init();
    log::info!("Listening on https://localhost:3030");

    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes)
        .tls()
        .cert_path("examples/tls/cert.pem")
        .key_path("examples/tls/key.rsa")
        .run(([127, 0, 0, 1], 3030))
        .await;
}

#[cfg(not(any(feature = "tls-openssl", feature = "tls")))]
fn main() {
    eprintln!("Requires the `tls` feature.");
}
