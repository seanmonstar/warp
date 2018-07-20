#![deny(warnings)]
extern crate warp;

#[test]
fn cookie() {
    let foo = warp::cookie("foo");

    let req = warp::test::request()
        .header("cookie", "foo=bar");
    assert_eq!(req.filter(&foo).unwrap(), "bar");

    let req = warp::test::request()
        .header("cookie", "abc=def; foo=baz");
    assert_eq!(req.filter(&foo).unwrap(), "baz");

    let req = warp::test::request()
        .header("cookie", "abc=def");
    assert!(!req.matches(&foo));

    let req = warp::test::request()
        .header("cookie", "foobar=quux");
    assert!(!req.matches(&foo));
}

#[test]
fn optional() {
    let foo = warp::cookie::optional("foo");

    let req = warp::test::request()
        .header("cookie", "foo=bar");
    assert_eq!(req.filter(&foo).unwrap().unwrap(), "bar");

    let req = warp::test::request()
        .header("cookie", "abc=def; foo=baz");
    assert_eq!(req.filter(&foo).unwrap().unwrap(), "baz");

    let req = warp::test::request()
        .header("cookie", "abc=def");
    assert!(req.matches(&foo));

    let req = warp::test::request()
        .header("cookie", "foobar=quux");
    assert!(req.matches(&foo));
}
