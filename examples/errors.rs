#![deny(warnings)]

extern crate pretty_env_logger;
extern crate warp;

use std::error::Error as StdError;
use std::fmt::{self, Display};

use warp::{Filter, reject, Rejection, Reply};
use warp::http::StatusCode;

#[derive(Debug)]
enum Error {
    Oops,
    NotFound
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
    let hello = warp::path::index()
        .map(warp::reply);

    let oops = warp::path("oops")
        .and_then(|| {
            Err::<StatusCode, _>(reject().with(Error::Oops))
        });

    let not_found = warp::path("not_found")
        .and_then(|| {
            Err::<StatusCode, _>(reject().with(Error::NotFound))
        });

    let routes = warp::get2()
        .and(hello.or(oops).or(not_found))
        .recover(customize_error);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}

// This function receives a `Rejection` and tries to return a custom
// value, othewise simply passes the rejection along.
//
// NOTE: We don't *need* to return an `impl Reply` here, it's just
// convenient in this specific case.
fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    let mut resp = err.json();

    let cause = match err.into_cause::<Error>() {
        Ok(ok) => ok,
        Err(err) => return Err(err)
    };

    match *cause {
        Error::NotFound => *resp.status_mut() = StatusCode::NOT_FOUND,
        Error::Oops => *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR,
    }

    Ok(resp)
}
