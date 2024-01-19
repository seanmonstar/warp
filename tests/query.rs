#![deny(warnings)]

use serde_derive::Deserialize;
use std::collections::HashMap;
use warp::Filter;

#[tokio::test]
async fn query() {
    let as_map = warp::query::<HashMap<String, String>>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_map).await.unwrap();
    assert_eq!(extracted["foo"], "bar");
    assert_eq!(extracted["baz"], "quux");
}

#[tokio::test]
async fn query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: Some("bar".into()),
            baz: Some("quux".into())
        }
    );
}

#[tokio::test]
async fn empty_query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/?");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: None,
            baz: None
        }
    );
}

#[tokio::test]
async fn query_struct_no_values() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/?foo&baz");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgs {
            foo: Some("".into()),
            baz: Some("".into())
        }
    );
}

#[tokio::test]
async fn missing_query_struct() {
    let as_struct = warp::query::<MyArgs>();

    let req = warp::test::request().path("/");

    let extracted = req.filter(&as_struct).await.unwrap();
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

#[tokio::test]
async fn required_query_struct() {
    let as_struct = warp::query::<MyRequiredArgs>();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyRequiredArgs {
            foo: "bar".into(),
            baz: "quux".into()
        }
    );
}

#[tokio::test]
async fn missing_required_query_struct_partial() {
    let as_struct = warp::query::<MyRequiredArgs>();

    let req = warp::test::request().path("/?foo=something");

    let extracted = req.filter(&as_struct).await;
    assert!(extracted.is_err())
}

#[tokio::test]
async fn missing_required_query_struct_no_query() {
    let as_struct = warp::query::<MyRequiredArgs>().map(|_| warp::reply());

    let req = warp::test::request().path("/");

    let res = req.reply(&as_struct).await;
    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "Invalid query string");
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct MyRequiredArgs {
    foo: String,
    baz: String,
}

#[tokio::test]
async fn raw_query() {
    let as_raw = warp::query::raw();

    let req = warp::test::request().path("/?foo=bar&baz=quux");

    let extracted = req.filter(&as_raw).await.unwrap();
    assert_eq!(extracted, "foo=bar&baz=quux".to_owned());
}

#[tokio::test]
async fn url_encoded_raw_query() {
    let as_raw = warp::query::raw();

    let req = warp::test::request().path("/?foo=bar%20hi&baz=quux");

    let extracted = req.filter(&as_raw).await.unwrap();
    assert_eq!(extracted, "foo=bar%20hi&baz=quux".to_owned());
}

#[tokio::test]
async fn plus_encoded_raw_query() {
    let as_raw = warp::query::raw();

    let req = warp::test::request().path("/?foo=bar+hi&baz=quux");

    let extracted = req.filter(&as_raw).await.unwrap();
    assert_eq!(extracted, "foo=bar+hi&baz=quux".to_owned());
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct MyArgsWithInt {
    string: Option<String>,
    number: Option<u32>,
}

#[tokio::test]
async fn query_struct_with_int() {
    let as_struct = warp::query::<MyArgsWithInt>();

    let req = warp::test::request().path("/?string=text&number=30");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgsWithInt {
            string: Some("text".into()),
            number: Some(30)
        }
    );
}

#[tokio::test]
async fn missing_query_struct_with_int() {
    let as_struct = warp::query::<MyArgsWithInt>();

    let req = warp::test::request().path("/");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgsWithInt {
            string: None,
            number: None
        }
    );
}

#[tokio::test]
async fn url_encoded_query_struct_with_int() {
    let as_struct = warp::query::<MyArgsWithInt>();

    let req = warp::test::request().path("/?string=test%20text&number=%33%30");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgsWithInt {
            string: Some("test text".into()),
            number: Some(30)
        }
    );
}

#[tokio::test]
async fn plus_encoded_query_struct() {
    let as_struct = warp::query::<MyArgsWithInt>();

    let req = warp::test::request().path("/?string=test+text");

    let extracted = req.filter(&as_struct).await.unwrap();
    assert_eq!(
        extracted,
        MyArgsWithInt {
            string: Some("test text".into()),
            number: None
        }
    );
}
