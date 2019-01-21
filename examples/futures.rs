#![deny(warnings)]
extern crate tokio;
extern crate warp;

use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::timer::Delay;
use warp::{Filter, Future};

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

fn main() {
    // Match `/:u32`...
    let routes = warp::path::param()
        // and_then create a `Future` that will simply wait N seconds...
        .and_then(|Seconds(seconds)| {
            Delay::new(Instant::now() + Duration::from_secs(seconds))
                // return the number of seconds again...
                .map(move |()| seconds)
                // An error from `Delay` means a big problem with the server...
                .map_err(|timer_err| {
                    eprintln!("timer error: {}", timer_err);
                    warp::reject::custom(timer_err)
                })
        })
        .map(|seconds| format!("I waited {} seconds!", seconds));

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
