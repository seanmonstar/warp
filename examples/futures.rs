#![deny(warnings)]

use std::str::FromStr;
use tokio::time::{Duration, delay_for};
use warp::Filter;

/// A newtype to enforce our maximum allowed seconds.
struct Seconds(u64);

impl FromStr for Seconds {
    type Err = ();
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        src.parse::<u64>().map_err(|_| ()).and_then(|num| {
            if num <= 5 {
                Ok(Seconds(num))
            } else {
                Err(())
            }
        })
    }
}

#[tokio::main]
async fn main() {
    // Match `/:Seconds`...
    let routes = warp::path::param()
        // and_then create a `Future` that will simply wait N seconds...
        .and_then(|Seconds(seconds): Seconds| async move {
            delay_for(Duration::from_secs(seconds)).await;
            Ok::<String, warp::Rejection>(format!("I waited {} seconds!", seconds))
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
