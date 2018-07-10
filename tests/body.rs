extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn concat() {
    let _ = pretty_env_logger::try_init();

    let concat = warp::body::concat();

    let req = warp::test::request()
        .path("/nothing-matches-me");

    assert!(req.matches(&concat));

    let p = warp::path("body");
    let req = warp::test::request()
        .path("/body");

    assert!(req.matches(&p.and(concat)));
}
