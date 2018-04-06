extern crate warp;

use warp::Filter;

#[test]
fn exact() {
    let foo = warp::path::exact("foo");
    let bar = warp::path::exact("bar");
    let foo_bar = foo.unit_and(bar);

    // /foo
    let mut req = warp::test::request()
        .path("/foo");

    assert!(foo.filter(req.route()).is_some());
    assert!(bar.filter(req.route()).is_none());
    assert!(foo_bar.filter(req.route()).is_none());

    // /foo/bar
    let mut req = warp::test::request()
        .path("/foo/bar");

    assert!(foo.filter(req.route()).is_some());
    assert!(bar.filter(req.route()).is_none());
    assert!(foo_bar.filter(req.route()).is_some());
}

#[test]
fn extract() {
    let num = warp::path::<u32>();

    let mut req = warp::test::request()
        .path("/321");
    assert_eq!(num.filter(req.route()).unwrap().1, 321);

    let s = warp::path::<String>();

    let mut req = warp::test::request()
        .path("/warp");
    assert_eq!(s.filter(req.route()).unwrap().1, "warp");
    // u32 doesn't extract a non-int
    assert!(num.filter(req.route()).is_none());

    let combo = num.map(|n| n + 5).and(s);

    let mut req = warp::test::request()
        .path("/42/vroom");
    assert_eq!(combo.filter(req.route()).unwrap().1, (47, "vroom".to_string()));
}

#[test]
fn or() {
    // /foo/bar OR /foo/baz
    let foo = warp::path::exact("foo");
    let bar = warp::path::exact("bar");
    let baz = warp::path::exact("baz");
    let p = foo.and(bar.or(baz));

    // /foo/bar
    let mut req = warp::test::request()
        .path("/foo/bar");

    assert!(p.filter(req.route()).is_some());

    // /foo/baz
    let mut req = warp::test::request()
        .path("/foo/baz");

    assert!(p.filter(req.route()).is_some());
}
