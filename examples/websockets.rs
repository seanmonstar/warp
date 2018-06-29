extern crate pretty_env_logger;
extern crate warp;

use warp::{Filter, Future, Stream};

fn main() {
    pretty_env_logger::init();

    let ws = warp::ws(|websocket| {
        // Just echo all messages back...
        let (tx, rx) = websocket.split();
        rx.forward(tx)
            .map(|_| ())
            .map_err(|e| {
                eprintln!("websocket error: {:?}", e);
            })
    });

    let path = warp::path::exact("chat");
    let routes = path.and(ws);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
