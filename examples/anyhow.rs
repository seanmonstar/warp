#![deny(warnings)]

use serde::export::Formatter;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

#[tokio::main]
async fn main() {
    let is_even = warp::path!("is_even" / u64).and_then(handler);

    let routes = warp::get().and(is_even).recover(handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handler(number: u64) -> anyhow::Result<impl Reply> {
    if number > 100 {
        anyhow::bail!(TooBig)
    }

    if number % 2 == 1 {
        anyhow::bail!(NotEven)
    }

    return Ok(StatusCode::OK);
}

#[derive(Debug)]
struct TooBig;

impl std::fmt::Display for TooBig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "sorry, we can't handle big numbers like this")
    }
}

#[derive(Debug)]
struct NotEven;

impl std::fmt::Display for NotEven {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "the given number is not even")
    }
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(anyhow) = err.find::<anyhow::Error>() {
        // here we can downcast the anyhow error to whatever we want
        if let Some(_) = anyhow.downcast_ref::<TooBig>() {
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }

        if let Some(_) = anyhow.downcast_ref::<NotEven>() {
            return Ok(StatusCode::BAD_REQUEST);
        }

        return Ok(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Err(err)
}
