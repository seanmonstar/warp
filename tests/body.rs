extern crate warp;

use warp::Filter;

#[test]
fn body_must_be_route_end() {
    let a = warp::body::concat();

    let mut req = warp::test::request()
        .path("/not-matched");

    assert!(a.filter(req.route()).is_none());

    let p = warp::path::exact("body");
    let mut req = warp::test::request()
        .path("/body");

    assert!(p.and(a).filter(req.route()).is_some());
}
