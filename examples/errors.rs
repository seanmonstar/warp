#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::{Filter, reject, Rejection, Reply};
use warp::http::{Response, StatusCode};

fn main() {
    let hello = warp::path::index()
        .map(warp::reply);

    let err500 = warp::path("500")
        .and_then(|| {
            Err::<StatusCode, _>(reject::server_error())
        });

    let routes = warp::get2()
        .and(hello.or(err500))
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
    match err.status() {
        StatusCode::NOT_FOUND => {
            // We have a custom 404 page!
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("you get a 404, and *you* get a 404..."))
        },
        StatusCode::INTERNAL_SERVER_ERROR => {
            // Oh no, something is on fire!
            eprintln!("quick, page someone! fire!");
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(":fire: this is fine"))
        }
        _ => {
            // Don't customize these errors, just let warp do
            // the default! (or optionally a later filter could
            // customize these).
            Err(err)
        }
    }
}
