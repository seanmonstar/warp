#![deny(warnings)]

use tokio::net::UnixListener;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let listener = UnixListener::bind("/tmp/warp.sock").unwrap();
    warp::serve(warp::fs::dir("examples/dir"))
        .run_incoming(listener)
        .await;
}
