#![deny(warnings)]
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate pretty_env_logger;
extern crate warp;

use std::error::Error as StdError;
use std::fmt::{self, Display};

use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

#[derive(Copy, Clone, Debug)]
enum Error {
    Oops,
    Nope,
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Error::Oops => ":fire: this is fine",
            Error::Nope => "Nope!",
        })
    }
}

impl StdError for Error {}

fn main() {
    let hello = warp::path::end().map(warp::reply);

    let oops =
        warp::path("oops").and_then(|| Err::<StatusCode, _>(warp::reject::custom(Error::Oops)));

    let nope =
        warp::path("nope").and_then(|| Err::<StatusCode, _>(warp::reject::custom(Error::Nope)));

    let routes = warp::get2()
        .and(hello.or(oops).or(nope))
        .recover(customize_error);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(&err) = err.find_cause::<Error>() {
        let code = match err {
            Error::Nope => StatusCode::BAD_REQUEST,
            Error::Oops => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let msg = err.to_string();

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });
        Ok(warp::reply::with_status(json, code))
    } else if let Some(_) = err.find_cause::<warp::reject::MethodNotAllowed>() {
        // We can handle a specific error, here METHOD_NOT_ALLOWED,
        // and render it however we want
        let code = StatusCode::METHOD_NOT_ALLOWED;
        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: "oops, you aren't allowed to use this method.".into(),
        });
        Ok(warp::reply::with_status(json, code))
    } else {
        // Could be a NOT_FOUND, or any other internal error... here we just
        // let warp use its default rendering.
        Err(err)
    }
}
