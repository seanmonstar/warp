extern crate warp;

use warp::Filter;

#[test]
fn matches() {
    let a = warp::body::concat();

    let req = warp::test::request();

    assert!(req.matches(&a));

    let p = warp::path("body");
    let req = warp::test::request()
        .path("/body");

    assert!(req.matches(&p.and(a)));
}
