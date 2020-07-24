#![deny(warnings)]

use tokio::net::TcpListener;
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut listener = TcpListener::bind("127.0.0.1:3030").await.unwrap();

    let routes = warp::get().map(warp::reply);
    warp::serve(routes)
        .serve_incoming(listener.incoming())
        .await;
}
