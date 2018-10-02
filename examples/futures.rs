#![deny(warnings)]
extern crate tokio;
extern crate warp;

use std::time::{Duration, Instant};
use tokio::timer::Delay;
use warp::{Filter, Future};

fn main() {
    // Match `/:u32`...
    let routes = warp::path::param()
        // Reject any that have too-high a number
        .and_then(|seconds: u64| {
            if seconds <= 5 {
                Ok(seconds)
            } else {
                Err(warp::reject())
            }
        })
        // and_then create a `Future` that will simply wait 3 seconds...
        .and_then(|seconds| {
            Delay::new(Instant::now() + Duration::from_secs(seconds))
                // return the number of seconds again...
                .map(move |()| seconds)
                // An error from `Delay` means a big problem with the server...
                .map_err(|timer_err| {
                    warp::reject::server_error().with(timer_err)
                })
        })
        .map(|seconds| format!("I waited {} seconds!", seconds));

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
