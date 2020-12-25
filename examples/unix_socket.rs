#![deny(warnings)]

use async_stream::stream;
use tokio::net::UnixListener;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let listener = UnixListener::bind("/tmp/warp.sock").unwrap();
    let incoming = stream! {
        while let item = listener.accept().await {
            yield item.map(|item| item.0);
        }
    };
    warp::serve(warp::fs::dir("examples/dir"))
        .run_incoming(incoming)
        .await;
}
