extern crate warp;
extern crate hyper;

use std::sync::Arc;
use warp::Filter;
use hyper::{Server, rt::Future};

fn main() {
    let addr = ([0, 0, 0, 0], 3030).into();

    let routes = warp::any().map(|| "ok");
    let routes = Arc::new(routes);
    let new_service = move || Ok::<_, hyper::Error>(routes.clone().into_service());

    let done = Server::bind(&addr)
        .serve(new_service)
        .map_err(|err| eprintln!("server error: {}", err));

    println!("Listening on http://{}", addr);
    hyper::rt::run(done);
}
