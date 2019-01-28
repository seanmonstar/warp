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
    let foo_req = || warp::test::request().path("/foo");

    assert!(foo_req().matches(&foo));
    assert!(!foo_req().matches(&bar));
    assert!(!foo_req().matches(&foo_bar));

    // /foo/bar
    let foo_bar_req = || warp::test::request().path("/foo/bar");

    assert!(foo_bar_req().matches(&foo));
    assert!(!foo_bar_req().matches(&bar));
    assert!(foo_bar_req().matches(&foo_bar));
}

#[test]
fn param() {
    let _ = pretty_env_logger::try_init();

    let num = warp::path::param::<u32>();

    let req = warp::test::request().path("/321");
    assert_eq!(req.filter(&num).unwrap(), 321);

    let s = warp::path::param::<String>();

    let req = warp::test::request().path("/warp");
    assert_eq!(req.filter(&s).unwrap(), "warp");

    // u32 doesn't extract a non-int
    let req = warp::test::request().path("/warp");
    assert!(!req.matches(&num));

    let combo = num.map(|n| n + 5).and(s);

    let req = warp::test::request().path("/42/vroom");
    assert_eq!(req.filter(&combo).unwrap(), (47, "vroom".to_string()));

    // empty segments never match
    let req = warp::test::request();
    assert!(
        !req.matches(&s),
        "param should never match an empty segment"
    );
}

#[test]
fn end() {
    let _ = pretty_env_logger::try_init();

    let foo = warp::path("foo");
    let end = warp::path::end();
    let foo_end = foo.and(end);

    assert!(
        warp::test::request()
            .path("/")
            .matches(&end),
        "end() matches /"
    );

    assert!(
        warp::test::request()
            .path("http://localhost:1234")
            .matches(&end),
        "end() matches /"
    );

    assert!(
        warp::test::request()
            .path("http://localhost:1234?q=2")
            .matches(&end),
        "end() matches empty path"
    );

    assert!(
        warp::test::request()
            .path("localhost:1234")
            .matches(&end),
        "end() matches authority-form"
    );

    assert!(
        !warp::test::request()
            .path("/foo")
            .matches(&end),
        "end() doesn't match /foo"
    );

    assert!(
        warp::test::request()
            .path("/foo")
            .matches(&foo_end),
        "path().and(end()) matches /foo"
    );

    assert!(
        warp::test::request()
            .path("/foo/")
            .matches(&foo_end),
        "path().and(end()) matches /foo/"
    );

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
    let ex = warp::test::request().path("/").filter(&tail).unwrap();
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
        .matches(&tail.and(warp::path::end())));
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
    let req = warp::test::request().path("/foo/bar");

    assert!(req.matches(&p));

    // /foo/baz
    let req = warp::test::request().path("/foo/baz");

    assert!(req.matches(&p));

    // deeper nested ORs
    // /foo/bar/baz OR /foo/baz/bar OR /foo/bar/bar
    let p = foo
        .and(bar.and(baz).map(|| panic!("shouldn't match")))
        .or(foo.and(baz.and(bar)).map(|| panic!("shouldn't match")))
        .or(foo.and(bar.and(bar)));

    // /foo/baz
    let req = warp::test::request().path("/foo/baz/baz");
    assert!(!req.matches(&p));

    // /foo/bar/bar
    let req = warp::test::request().path("/foo/bar/bar");
    assert!(req.matches(&p));
}

#[test]
fn or_else() {
    let _ = pretty_env_logger::try_init();

    let foo = warp::path("foo");
    let bar = warp::path("bar");

    let p = foo.and(bar.or_else(|_| Ok(())));

    // /foo/bar
    let req = warp::test::request().path("/foo/nope");

    assert!(req.matches(&p));
}

#[test]
fn path_macro() {
    let _ = pretty_env_logger::try_init();

    let req = warp::test::request().path("/foo/bar");
    let p = path!("foo" / "bar");
    assert!(req.matches(&p));

    let req = warp::test::request().path("/foo/bar");
    let p = path!(String / "bar");
    assert_eq!(req.filter(&p).unwrap(), "foo");

    let req = warp::test::request().path("/foo/bar");
    let p = path!("foo" / String);
    assert_eq!(req.filter(&p).unwrap(), "bar");
}

#[test]
fn full_path() {
    let full_path = warp::path::full();

    let foo = warp::path("foo");
    let bar = warp::path("bar");
    let param = warp::path::param::<u32>();

    // matches full request path
    let ex = warp::test::request()
        .path("/42/vroom")
        .filter(&full_path)
        .unwrap();
    assert_eq!(ex.as_str(), "/42/vroom");

    // matches index
    let ex = warp::test::request().path("/").filter(&full_path).unwrap();
    assert_eq!(ex.as_str(), "/");

    // does not include query
    let ex = warp::test::request()
        .path("/foo/bar?baz=quux")
        .filter(&full_path)
        .unwrap();
    assert_eq!(ex.as_str(), "/foo/bar");

    // includes previously matched prefix
    let ex = warp::test::request()
        .path("/foo/bar")
        .filter(&foo.and(full_path))
        .unwrap();
    assert_eq!(ex.as_str(), "/foo/bar");

    // includes following matches
    let ex = warp::test::request()
        .path("/foo/bar")
        .filter(&full_path.and(foo))
        .unwrap();
    assert_eq!(ex.as_str(), "/foo/bar");

    // includes previously matched param
    let (_, ex) = warp::test::request()
        .path("/foo/123")
        .filter(&foo.and(param).and(full_path))
        .unwrap();
    assert_eq!(ex.as_str(), "/foo/123");

    // does not modify matching
    assert!(warp::test::request()
        .path("/foo/bar")
        .matches(&full_path.and(foo).and(bar)));
}

#[test]
fn peek() {
    let peek = warp::path::peek();

    let foo = warp::path("foo");
    let bar = warp::path("bar");
    let param = warp::path::param::<u32>();

    // matches full request path
    let ex = warp::test::request()
        .path("/42/vroom")
        .filter(&peek)
        .unwrap();
    assert_eq!(ex.as_str(), "42/vroom");

    // matches index
    let ex = warp::test::request().path("/").filter(&peek).unwrap();
    assert_eq!(ex.as_str(), "");

    // does not include query
    let ex = warp::test::request()
        .path("/foo/bar?baz=quux")
        .filter(&peek)
        .unwrap();
    assert_eq!(ex.as_str(), "foo/bar");

    // does not include previously matched prefix
    let ex = warp::test::request()
        .path("/foo/bar")
        .filter(&foo.and(peek))
        .unwrap();
    assert_eq!(ex.as_str(), "bar");

    // includes following matches
    let ex = warp::test::request()
        .path("/foo/bar")
        .filter(&peek.and(foo))
        .unwrap();
    assert_eq!(ex.as_str(), "foo/bar");

    // does not include previously matched param
    let (_, ex) = warp::test::request()
        .path("/foo/123")
        .filter(&foo.and(param).and(peek))
        .unwrap();
    assert_eq!(ex.as_str(), "");

    // does not modify matching
    assert!(warp::test::request()
        .path("/foo/bar")
        .matches(&peek.and(foo).and(bar)));
}

#[test]
fn peek_segments() {
    let peek = warp::path::peek();

    // matches full request path
    let ex = warp::test::request()
        .path("/42/vroom")
        .filter(&peek)
        .unwrap();

    assert_eq!(ex.segments().collect::<Vec<_>>(), &["42", "vroom"]);

    // matches index
    let ex = warp::test::request().path("/").filter(&peek).unwrap();

    let segs = ex.segments().collect::<Vec<_>>();
    assert_eq!(segs, Vec::<&str>::new());
}

