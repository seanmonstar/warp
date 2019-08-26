use std::time::Duration;
use tokio::{clock::now, timer::Interval};
use futures::{never::Never, StreamExt};
use warp::{Filter, sse::ServerSentEvent};

// create server-sent event
fn sse_counter(counter: u64) ->  Result<impl ServerSentEvent, Never> {
    Ok(warp::sse::data(counter))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let routes = warp::path("ticks")
        .and(warp::sse())
        .map(|sse: warp::sse::Sse| {
            let mut counter: u64 = 0;
            // create server event source
            let event_stream = Interval::new(now(), Duration::from_secs(1)).map(move |_| {
                counter += 1;
                sse_counter(counter)
            });
            // reply using server-sent events
            sse.reply(event_stream)
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
