#![deny(warnings)]

use warp::Filter;
use futures::{FutureExt, StreamExt};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let routes = warp::path("echo")
        // The `ws2()` filter will prepare the Websocket handshake.
        .and(warp::ws2())
        .map(|ws: warp::ws::Ws2| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
            })
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
