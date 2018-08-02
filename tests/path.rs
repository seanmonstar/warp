#![deny(warnings)]
extern crate pretty_env_logger;
#[macro_use]
extern crate warp;

use warp::Filter;

#[test]
fn path() {
    let _ = pretty_env_logger::try_init();

    let foo = warp::path("foo");
    let bar = warp::path("bar");
    let foo_bar = foo.and(bar);

    // /foo
    let foo_req = || {
        warp::test::request()
            .path("/foo")
    };

    assert!(foo_req().matches(&foo));
    assert!(!foo_req().matches(&bar));
    assert!(!foo_req().matches(&foo_bar));


    // /foo/bar
    let foo_bar_req = || {
        warp::test::request()
            .path("/foo/bar")
    };

    assert!(foo_bar_req().matches(&foo));
    assert!(!foo_bar_req().matches(&bar));
    assert!(foo_bar_req().matches(&foo_bar));
}

#[test]
fn param() {
    let _ = pretty_env_logger::try_init();

    let num = warp::path::param::<u32>();

    let req = warp::test::request()
        .path("/321");
    assert_eq!(req.filter(&num).unwrap(), 321);

    let s = warp::path::param::<String>();

    let req = warp::test::request()
        .path("/warp");
    assert_eq!(req.filter(&s).unwrap(), "warp");

    // u32 doesn't extract a non-int
    let req = warp::test::request()
        .path("/warp");
    assert!(!req.matches(&num));

    let combo = num.map(|n| n + 5).and(s);

    let req = warp::test::request()
        .path("/42/vroom");
    assert_eq!(req.filter(&combo).unwrap(), (47, "vroom".to_string()));

    // empty segments never match
    let req = warp::test::request();
    assert!(!req.matches(&s), "param should never match an empty segment");
}

#[test]
fn tail() {
    let tail = warp::path::tail();

    // matches full path
    let ex = warp::test::request()
        .path("/42/vroom")
        .filter(&tail)
        .unwrap();
    assert_eq!(ex.as_str(), "42/vroom");

    // matches index
    let ex = warp::test::request()
        .path("/")
        .filter(&tail)
        .unwrap();
    assert_eq!(ex.as_str(), "");

    // doesn't include query
    let ex = warp::test::request()
        .path("/foo/bar?baz=quux")
        .filter(&tail)
        .unwrap();
    assert_eq!(ex.as_str(), "foo/bar");

    // doesn't include previously matched prefix
    let ex = warp::test::request()
        .path("/foo/bar")
        .filter(&warp::path("foo").and(tail))
        .unwrap();
    assert_eq!(ex.as_str(), "bar");

    // sets unmatched path index to end
    assert!(!warp::test::request()
        .path("/foo/bar")
        .matches(&tail.and(warp::path("foo"))));

    assert!(warp::test::request()
        .path("/foo/bar")
        .matches(&tail.and(warp::path::index())));
}

#[test]
fn or() {
    let _ = pretty_env_logger::try_init();

    // /foo/bar OR /foo/baz
    let foo = warp::path("foo");
    let bar = warp::path("bar");
    let baz = warp::path("baz");
    let p = foo.and(bar.or(baz));

    // /foo/bar
    let req = warp::test::request()
        .path("/foo/bar");

    assert!(req.matches(&p));

    // /foo/baz
    let req = warp::test::request()
        .path("/foo/baz");

    assert!(req.matches(&p));

    // deeper nested ORs
    // /foo/bar/baz OR /foo/baz/bar OR /foo/bar/bar
    let p = foo.and(bar.and(baz).map(|| panic!("shouldn't match")))
        .or(foo.and(baz.and(bar)).map(|| panic!("shouldn't match")))
        .or(foo.and(bar.and(bar)));

    // /foo/baz
    let req = warp::test::request()
        .path("/foo/baz/baz");
    assert!(!req.matches(&p));


    // /foo/bar/bar
    let req = warp::test::request()
        .path("/foo/bar/bar");
    assert!(req.matches(&p));
}

#[test]
fn or_else() {
    let _ = pretty_env_logger::try_init();

    let foo = warp::path("foo");
    let bar = warp::path("bar");

    let p = foo.and(bar.or_else(|_| Ok(())));

    // /foo/bar
    let req = warp::test::request()
        .path("/foo/nope");

    assert!(req.matches(&p));
}

#[test]
fn path_macro() {
    let _ = pretty_env_logger::try_init();

    let req = warp::test::request()
        .path("/foo/bar");
    let p = path!("foo" / "bar");
    assert!(req.matches(&p));

    let req = warp::test::request()
        .path("/foo/bar");
    let p = path!(String / "bar");
    assert_eq!(req.filter(&p).unwrap(), "foo");

    let req = warp::test::request()
        .path("/foo/bar");
    let p = path!("foo" / String);
    assert_eq!(req.filter(&p).unwrap(), "bar");
}

