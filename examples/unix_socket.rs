#![deny(warnings)]

use async_stream::try_stream;
use std::io;
use tokio::net::{unix::SocketAddr, UnixListener, UnixStream};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let listener = UnixListener::bind("/tmp/warp.sock").unwrap();
    let stream = try_stream! {
        let rslt: io::Result<(UnixStream, SocketAddr)> = listener.accept().await;
        while let (socket, _) = listener.accept().await? {
            yield socket;
        }
    };
    warp::serve(warp::fs::dir("examples/dir"))
        .run_incoming(stream)
        .await;
}
