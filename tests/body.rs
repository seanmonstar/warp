extern crate warp;

use warp::Filter;

#[test]
fn body_must_be_route_end() {
    let a = warp::body::concat();

    let req = warp::test::request()
        .path("/not-matched");

    assert!(!req.matches(&a));

    let p = warp::path::exact("body");
    let req = warp::test::request()
        .path("/body");

    assert!(req.matches(&p.and(a)));
}
