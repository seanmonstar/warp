#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

#[test]
fn cookie() {
    let foo = warp::cookie("foo");

    let req = warp::test::request().header("cookie", "foo=bar");
    assert_eq!(req.filter(&foo).unwrap(), "bar");

    let req = warp::test::request().header("cookie", "abc=def; foo=baz");
    assert_eq!(req.filter(&foo).unwrap(), "baz");

    let req = warp::test::request().header("cookie", "abc=def");
    assert!(!req.matches(&foo));

    let req = warp::test::request().header("cookie", "foobar=quux");
    assert!(!req.matches(&foo));
}

#[test]
fn optional() {
    let foo = warp::cookie::optional("foo");

    let req = warp::test::request().header("cookie", "foo=bar");
    assert_eq!(req.filter(&foo).unwrap().unwrap(), "bar");

    let req = warp::test::request().header("cookie", "abc=def; foo=baz");
    assert_eq!(req.filter(&foo).unwrap().unwrap(), "baz");

    let req = warp::test::request().header("cookie", "abc=def");
    assert!(req.matches(&foo));

    let req = warp::test::request().header("cookie", "foobar=quux");
    assert!(req.matches(&foo));
}

#[test]
fn missing() {
    let _ = pretty_env_logger::try_init();

    let cookie = warp::cookie("foo");

    let res = warp::test::request()
        .header("cookie", "not=here")
        .reply(&cookie);

    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "Missing request cookie 'foo'");
}
