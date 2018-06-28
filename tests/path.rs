extern crate warp;

use warp::Filter;

#[test]
fn exact() {
    let foo = warp::path::exact("foo");
    let bar = warp::path::exact("bar");
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
fn extract() {
    let num = warp::path::<u32>();

    let req = warp::test::request()
        .path("/321");
    assert_eq!(req.filter(num), Some(321));

    let s = warp::path::<String>();

    let req = warp::test::request()
        .path("/warp");
    assert_eq!(req.filter(s).unwrap(), "warp");

    // u32 doesn't extract a non-int
    let req = warp::test::request()
        .path("/warp");
    assert_eq!(req.filter(num), None);

    let combo = num.map(|n| n + 5).and(s);

    let req = warp::test::request()
        .path("/42/vroom");
    assert_eq!(req.filter(combo), Some((47, "vroom".to_string())));
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

    assert!(req.matches(p));

    // /foo/baz
    let req = warp::test::request()
        .path("/foo/baz");

    assert!(req.matches(p));

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
