#![deny(warnings)]
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate pretty_env_logger;
extern crate warp;

use warp::http::StatusCode;
use warp::{reject, Filter, Rejection, Reply};

/// A custom `Reject` type.
#[derive(Debug)]
enum Error {
    Oops,
    Nope,
}

impl reject::Reject for Error {}

/// A serialized message to report in JSON format.
#[derive(Serialize)]
struct ErrorMessage<'a> {
    code: u16,
    message: &'a str,
}

fn main() {
    let hello = warp::path::end().map(warp::reply);

    let oops = warp::path("oops").and_then(|| {
        Err::<StatusCode, _>(reject::custom(Error::Oops))
    });

    let nope = warp::path("nope").and_then(|| {
        Err::<StatusCode, _>(reject::custom(Error::Nope))
    });

    let routes = warp::get()
        .and(hello.or(oops).or(nope))
        .recover(customize_error);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(err) = err.find::<Error>() {
        let (code, msg) = match err {
            Error::Nope => (StatusCode::BAD_REQUEST, "Nope!"),
            Error::Oops => (StatusCode::INTERNAL_SERVER_ERROR, ":fire: this is fine"),
        };

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });
        Ok(warp::reply::with_status(json, code))
    } else {
        // Could be a NOT_FOUND, or METHOD_NOT_ALLOWED... here we just
        // let warp use its default rendering.
        Err(err)
    }
}
