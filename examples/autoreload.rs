#![deny(warnings)]
use hyper::server::Server;
use listenfd::ListenFd;
use std::convert::Infallible;
use warp::Filter;

extern crate listenfd;
/// You'll need to install `systemfd` and `cargo-watch`:
/// ```
/// cargo install systemfd cargo-watch
/// ```
/// And run with:
/// ```
/// systemfd --no-pid -s http::3030 -- cargo watch -x 'run --example autoreload'
/// ```
#[tokio::main]
async fn main() {
    // Get README.md file for every first level path
    let readme = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("./README.md"));

    // Get examples directory if the path match with ex
    let examples = warp::path("ex").and(warp::fs::dir("./examples/"));

    let routes = readme.or(examples);

    // hyper let's us build a server from a TcpListener (which will be
    // useful shortly). Thus, we'll need to convert our `warp::Filter` into
    // a `hyper::service::MakeService` for use with a `hyper::server::Server`.
    let svc = warp::service(routes);
    let make_svc = hyper::service::make_service_fn(|_: _| {
        // clone svc is needed to avoid the error move out of svc
        let svc = svc.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = ListenFd::from_env();
    // if listenfd doesn't take a TcpListener (i.e. we're not running via
    // the command above), we fall back to explicitly binding to a given
    // host:port.
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        Server::from_tcp(l).unwrap()
    } else {
        Server::bind(&([127, 0, 0, 1], 3030).into())
    };

    server.serve(make_svc).await.unwrap();
}
