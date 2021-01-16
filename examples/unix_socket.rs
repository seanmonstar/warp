#![deny(warnings)]

use futures::TryStreamExt;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use warp::LiftIo;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let listener = UnixListener::bind("/tmp/warp.sock").unwrap();
    let incoming = UnixListenerStream::new(listener)
        .map_ok(LiftIo)
        .into_stream();
    warp::serve(warp::fs::dir("examples/dir"))
        .run_incoming(incoming)
        .await;
}
