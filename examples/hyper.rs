extern crate warp;
extern crate hyper;

use std::sync::Arc;
use warp::Filter;
use hyper::{Server, rt::Future};

fn main() {
    let addr = ([0, 0, 0, 0], 3030).into();
    let routes = Arc::new(warp::any().map(|| "ok"));
    let new_service = move || Ok::<_, hyper::Error>(routes.clone().lift());

    let done = Server::bind(&addr)
        .serve(new_service)
        .map_err(|err| eprintln!("server error: {}", err));

    println!("Listening on http://{}", addr);
    hyper::rt::run(done);
}
