#![deny(warnings)]

use futures::TryStreamExt;
use tokio::net::UnixListener;
use warp::LiftIo;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut listener = UnixListener::bind("/tmp/warp.sock").unwrap();
    let incoming = listener.incoming().map_ok(LiftIo).into_stream();
    warp::serve(warp::fs::dir("examples/dir"))
        .run_incoming(incoming)
        .await;
}
