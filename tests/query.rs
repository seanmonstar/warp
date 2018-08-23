#![deny(warnings)]
extern crate warp;

use std::collections::HashMap;

#[test]
fn query() {
    let as_map = warp::query::<HashMap<String, String>>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_map).unwrap();
    assert_eq!(extracted["foo"], "bar");
    assert_eq!(extracted["baz"], "quux");
}

#[test]
fn raw_query() {
    let as_raw = warp::query::raw();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_raw).unwrap();
    assert_eq!(extracted, "foo=bar&baz=quux".to_owned());
}
