#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::{http::Method, Filter};

#[test]
fn allow_methods() {
    let cors = warp::cors().allow_methods(&[Method::GET, Method::POST, Method::DELETE]);

    let route = warp::any().map(warp::reply).with(cors);

    let res = warp::test::request()
        .method("OPTIONS")
        .header("origin", "warp")
        .header("access-control-request-method", "DELETE")
        .reply(&route);

    assert_eq!(res.status(), 200);

    let res = warp::test::request()
        .method("OPTIONS")
        .header("origin", "warp")
        .header("access-control-request-method", "PUT")
        .reply(&route);

    assert_eq!(res.status(), 403);
}

#[test]
fn origin_not_allowed() {
    let cors = warp::cors()
        .allow_methods(&[Method::DELETE])
        .allow_origin("https://hyper.rs");

    let route = warp::any().map(warp::reply).with(cors);

    let res = warp::test::request()
        .method("OPTIONS")
        .header("origin", "https://warp.rs")
        .header("access-control-request-method", "DELETE")
        .reply(&route);

    assert_eq!(res.status(), 403);

    let res = warp::test::request()
        .header("origin", "https://warp.rs")
        .header("access-control-request-method", "DELETE")
        .reply(&route);

    assert_eq!(res.status(), 403);
}

#[test]
fn headers_not_allowed() {
    let cors = warp::cors()
        .allow_methods(&[Method::DELETE])
        .allow_headers(vec!["x-foo"]);

    let route = warp::any().map(warp::reply).with(cors);

    let res = warp::test::request()
        .method("OPTIONS")
        .header("origin", "https://warp.rs")
        .header("access-control-request-headers", "x-bar")
        .header("access-control-request-method", "DELETE")
        .reply(&route);

    assert_eq!(res.status(), 403);
}

#[test]
fn success() {
    let cors = warp::cors()
        .allow_credentials(true)
        .allow_headers(vec!["x-foo", "x-bar"])
        .allow_methods(&[Method::POST, Method::DELETE])
        .max_age(30);

    let route = warp::any().map(warp::reply).with(cors);

    // preflight
    let res = warp::test::request()
        .method("OPTIONS")
        .header("origin", "https://hyper.rs")
        .header("access-control-request-headers", "x-bar,x-foo")
        .header("access-control-request-method", "DELETE")
        .reply(&route);
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()["access-control-allow-origin"],
        "https://hyper.rs"
    );
    assert_eq!(res.headers()["access-control-allow-credentials"], "true");
    let headers = &res.headers()["access-control-allow-headers"];
    assert!(headers == "x-bar, x-foo" || headers == "x-foo, x-bar");
    assert_eq!(res.headers()["access-control-max-age"], "30");
    let methods = &res.headers()["access-control-allow-methods"];
    assert!(
        // HashSet randomly orders these...
        methods == "DELETE, POST" || methods == "POST, DELETE",
        "access-control-allow-methods: {:?}",
        methods,
    );

    // cors request
    let res = warp::test::request()
        .method("DELETE")
        .header("origin", "https://hyper.rs")
        .header("x-foo", "hello")
        .header("x-bar", "world")
        .reply(&route);
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()["access-control-allow-origin"],
        "https://hyper.rs"
    );
    assert_eq!(res.headers()["access-control-allow-credentials"], "true");
    assert_eq!(res.headers().get("access-control-max-age"), None);
    assert_eq!(res.headers().get("access-control-allow-methods"), None);
}
