extern crate pretty_env_logger;
extern crate tokio;
extern crate warp;

use std::time::Duration;
use tokio::{clock::now, timer::Interval};
use warp::{Filter, Stream};

fn main() {
    pretty_env_logger::init();

    let routes = warp::path("ticks")
        .and(warp::sse())
        .map(|sse: warp::sse::Sse| {
            let mut counter: u64 = 0;
            // create server event source
            let event_stream = Interval::new(now(), Duration::from_secs(1)).map(move |_| {
                counter += 1;
                // create server-sent event
                warp::sse::data(counter)
            });
            // reply using server-sent events
            sse.reply(event_stream)
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
