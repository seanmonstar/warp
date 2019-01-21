#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn exact() {
    let _ = pretty_env_logger::try_init();

    let host = warp::header::exact("host", "localhost");

    let req = warp::test::request().header("host", "localhost");

    assert!(req.matches(&host));

    let req = warp::test::request();
    assert!(!req.matches(&host), "header missing");

    let req = warp::test::request().header("host", "hyper.rs");
    assert!(!req.matches(&host), "header value different");
}

#[test]
fn exact_rejections() {
    let _ = pretty_env_logger::try_init();

    let host = warp::header::exact("host", "localhost").map(warp::reply);

    let res = warp::test::request().header("host", "nope").reply(&host);

    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "Invalid request header 'host'");

    let res = warp::test::request()
        .header("not-even-a-host", "localhost")
        .reply(&host);

    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "Missing request header 'host'");
}
