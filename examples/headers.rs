#![deny(warnings)]
use std::net::SocketAddr;
use warp::{Filter, Reply};

/// Create a server that requires header conditions:
///
/// - `Host` is a `SocketAddr`
/// - `Accept` is exactly `*/*`
///
/// If these conditions don't match, a 404 is returned.
///
/// Try it out with `curl -v -H "Accept: */*" 127.0.0.1:3030`
#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // For this example, we assume no DNS was used,
    // so the Host header should be an address.
    let host = warp::header::<SocketAddr>("host");

    // Match when we get `accept: */*` exactly.
    let accept_stars = warp::header::exact("accept", "*/*");

    let routes = host
        .and(accept_stars)
        .map(|addr| format!("accepting stars on {}", addr).with_header("server", "warp")); // Reply with a `server` header

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
