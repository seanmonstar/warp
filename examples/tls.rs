#![deny(warnings)]
extern crate warp;

// Don't copy this `cfg`, it's only needed because this file is within
// the warp repository.
#[cfg(feature = "tls")]
fn main() {
    use warp::Filter;

    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");

    warp::serve(routes)
        .tls("examples/tls/cert.pem", "examples/tls/key.rsa")
        .run(([127, 0, 0, 1], 3030));
}

#[cfg(not(feature = "tls"))]
fn main() {
    eprintln!("Requires the `tls` feature.");
}
