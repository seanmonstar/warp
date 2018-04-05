extern crate warp;

use warp::Filter;

#[test]
fn exact() {
    let foo = warp::path::exact("foo");
    let bar = warp::path::exact("bar");
    let foo_bar = foo.unit_and(bar);

    // /foo
    let req = warp::test::request()
        .path("/foo");

    assert!(foo.filter(req.route()).is_some());
    assert!(bar.filter(req.route()).is_none());
    assert!(foo_bar.filter(req.route()).is_none());

    // /foo/bar
    let req = warp::test::request()
        .path("/foo/bar");

    assert!(foo.filter(req.route()).is_some());
    assert!(bar.filter(req.route()).is_none());
    assert!(foo_bar.filter(req.route()).is_some());
}

#[test]
fn or() {
    // /foo/bar OR /foo/baz
    let foo = warp::path::exact("foo");
    let bar = warp::path::exact("bar");
    let baz = warp::path::exact("baz");
    let p = foo.and(bar.or(baz));

    // /foo/bar
    let req = warp::test::request()
        .path("/foo/bar");

    assert!(p.filter(req.route()).is_some());

    // /foo/baz
    let req = warp::test::request()
        .path("/foo/baz");

    assert!(p.filter(req.route()).is_some());
}
