#![deny(warnings)]
extern crate warp;

use warp::{filters::method::v2 as methodv2, Filter};

#[test]
fn method() {
    let get = warp::get(warp::any().map(warp::reply));

    let req = warp::test::request();
    assert!(req.matches(&get));

    let req = warp::test::request()
        .method("POST");
    assert!(!req.matches(&get));

    let req = warp::test::request()
        .method("POST");
    let resp = req.reply(&get);
    assert_eq!(resp.status(), 405);
}

#[test]
fn methodv2() {
    let get = warp::any().map(warp::reply)
        .with(methodv2::get());

    let req = warp::test::request();
    assert!(req.matches(&get));

    let req = warp::test::request()
        .method("POST");
    assert!(!req.matches(&get));

    let req = warp::test::request()
        .method("POST");
    let resp = req.reply(&get);
    assert_eq!(resp.status(), 405);
}

#[test]
fn cancels_method_not_allowed() {
    let get = warp::get(warp::path("hello").map(warp::reply));
    let post = warp::post(warp::path("bye").map(warp::reply));

    let routes = get.or(post);


    let req = warp::test::request()
        .method("GET")
        .path("/bye");

    let resp = req.reply(&routes);
    // A GET was allowed, but the path was wrong, so it should return
    // a 404, not a 405.
    assert_eq!(resp.status(), 404);
}

#[test]
fn v2_cancels_method_not_allowed() {
    let get = warp::path("hello").map(warp::reply)
        .with(methodv2::get());
    let post = warp::path("bye").map(warp::reply)
        .with(methodv2::post());

    let routes = get.or(post);


    let req = warp::test::request()
        .method("GET")
        .path("/bye");

    let resp = req.reply(&routes);
    // A GET was allowed, but the path was wrong, so it should return
    // a 404, not a 405.
    assert_eq!(resp.status(), 404);
}
