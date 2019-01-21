#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::{http::Uri, Filter};

#[test]
fn redirect_uri() {
    let over_there = warp::any().map(|| warp::redirect(Uri::from_static("/over-there")));

    let req = warp::test::request();
    let resp = req.reply(&over_there);

    assert_eq!(resp.status(), 301);
    assert_eq!(resp.headers()["location"], "/over-there");
}
