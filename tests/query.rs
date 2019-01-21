#![deny(warnings)]
extern crate warp;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use warp::Filter;

#[test]
fn query() {
    let as_map = warp::query::<HashMap<String, String>>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_map).unwrap();
    assert_eq!(extracted["foo"], "bar");
    assert_eq!(extracted["baz"], "quux");
}

#[test]
fn query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_struct).unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: Some("bar".into()),
            baz: Some("quux".into())
        }
    );
}

#[test]
fn empty_query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/?");

    let extracted = req.filter(&as_struct).unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: None,
            baz: None
        }
    );
}

#[test]
fn missing_query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/");

    let extracted = req.filter(&as_struct).unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: None,
            baz: None
        }
    );
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct MyArgs {
    foo: Option<String>,
    baz: Option<String>,
}

#[test]
fn required_query_struct() {
    let as_struct = warp::query::<MyRequiredArgs>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_struct).unwrap();
    assert_eq!(
        extracted,
        MyRequiredArgs {
            foo: "bar".into(),
            baz: "quux".into()
        }
    );
}

#[test]
fn missing_required_query_struct_partial() {
    let as_struct = warp::query::<MyRequiredArgs>();

    let req = warp::test::request().path("/?foo=something");

    let extracted = req.filter(&as_struct);
    assert!(extracted.is_err())
}

#[test]
fn missing_required_query_struct_no_query() {
    let as_struct = warp::query::<MyRequiredArgs>().map(|_| warp::reply());

    let req = warp::test::request().path("/");

    let res = req.reply(&as_struct);
    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "Invalid query string");
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct MyRequiredArgs {
    foo: String,
    baz: String,
}

#[test]
fn raw_query() {
    let as_raw = warp::query::raw();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_raw).unwrap();
    assert_eq!(extracted, "foo=bar&baz=quux".to_owned());
}
