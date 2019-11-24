extern crate warp;
extern crate hyper;

use warp::Filter;
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};
use tower_service::Service;

#[tokio::main]
async fn main() {
    let addr = ([0, 0, 0, 0], 3030).into();

    let routes = warp::any().map(|| "hello world");

    let make_service = make_service_fn(move |_| {
            let routes = routes.clone();
            futures::future::ok::<_, hyper::Error>(service_fn(move |req| routes.into_service().call(req)))
        });

    let server = Server::bind(&addr)
        .serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
