extern crate pretty_env_logger;
extern crate warp;

use std::net::SocketAddr;
use warp::Filter;

/// Create a server that requires header conditions:
///
/// - `Host` is a `SocketAddr`
/// - `Accept` is exactly `*/*`
///
/// If these conditions don't match, a 404 is returned.
fn main() {
    pretty_env_logger::init();

    // For this example, we assume no DNS was used,
    // so the Host header should be an address.
    let host = warp::header::<SocketAddr>("host");

    // Match when we get `accept: */*` exactly.
    let accept_stars = warp::header::exact("accept", "*/*");


    // and_unit removes the () from accept_stars...
    // With trait specialization, it should no longer be needed.
    let index = host.and_unit(accept_stars)
        .map(|addr| {
            format!("accepting stars on {}", addr)
        });

    // Only allow GETs in this example
    let routes = warp::get(
        index
            // Map the String to a Response...
            // With trait specialization, it should no longer be needed.
            .map(warp::reply)
    );

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}

