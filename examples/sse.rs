use futures::StreamExt;
use std::convert::Infallible;
use std::time::Duration;
use tokio::time::interval;
use warp::{sse::Event, Filter};

// create server-sent event
fn sse_counter(counter: u64) -> Result<Event, Infallible> {
    Ok(warp::sse::Event::default().data(Some(counter)))
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let routes = warp::path("ticks").and(warp::get()).map(|| {
        let mut counter: u64 = 0;
        // create server event source
        let event_stream = interval(Duration::from_secs(1)).map(move |_| {
            counter += 1;
            sse_counter(counter)
        });
        // reply using server-sent events
        warp::sse::reply(event_stream)
    });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
