#![deny(warnings)]
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate pretty_env_logger;
extern crate warp;

use std::error::Error as StdError;
use std::fmt::{self, Display};

use warp::{Filter, Rejection, Reply};
use warp::http::StatusCode;

#[derive(Copy, Clone, Debug)]
enum Error {
    Oops,
    NotFound
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::Oops => ":fire: this is fine",
            Error::NotFound => "you get a 404, and *you* get a 404...",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        None
    }
}


fn main() {
    let hello = warp::path::end()
        .map(warp::reply);

    let oops = warp::path("oops")
        .and_then(|| {
            Err::<StatusCode, _>(warp::reject::custom(Error::Oops))
        });

    let not_found = warp::path("not_found")
        .and_then(|| {
            Err::<StatusCode, _>(warp::reject::custom(Error::NotFound))
        });

    let routes = warp::get2()
        .and(hello.or(oops).or(not_found))
        .recover(customize_error);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(&err) = err.find_cause::<Error>() {
        let code = match err {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::Oops => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let msg = err.to_string();

        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: msg,
        });
        Ok(warp::reply::with_status(json, code))
    } else {
        Err(err)
    }
    /*
    let (code, msg) = match err.find_cause::<Error>() {
        Some(&Error::NotFound) => StatusCode::NOT_FOUND,
        Some(&Error::Oops) => StatusCode::INTERNAL_SERVER_ERROR,
        None => return Err(err),
    };
    */
}
