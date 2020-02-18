#![deny(warnings)]
use std::convert::Infallible;
use warp::Filter;
use hyper::server::Server;
use listenfd::ListenFd;

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
    // Match any request and return hello world!
    let routes = warp::any().map(|| "Hello, World!");

    // Convert warp filter into a hyper service, allowing us to later use
    // it with `hyper::Server::from_tcp(...).serve(make_svc)`.
    let svc = warp::service(routes);
    let make_svc = hyper::service::make_service_fn(|_: _| async move {
        Ok::<_, Infallible>(svc)
    });

    let mut listenfd = ListenFd::from_env();
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        Server::from_tcp(l).unwrap()
    } else {
        Server::bind(&([127,0,0,1], 3030).into())
    };

    server.serve(make_svc).await.unwrap();
}
