extern crate pretty_env_logger;
extern crate warp;

#[test]
fn exact() {
    let _ = pretty_env_logger::try_init();

    let host = warp::header::exact("host", "localhost");

    let req = warp::test::request()
        .header("host", "localhost");

    assert!(req.matches(&host));

    let req = warp::test::request();
    assert!(!req.matches(&host), "header missing");


    let req = warp::test::request()
        .header("host", "hyper.rs");
    assert!(!req.matches(&host), "header value different");
}
