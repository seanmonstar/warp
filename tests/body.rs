#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn matches() {
    let _ = pretty_env_logger::try_init();

    let concat = warp::body::concat();

    let req = warp::test::request()
        .path("/nothing-matches-me");

    assert!(req.matches(&concat));

    let p = warp::path("body");
    let req = warp::test::request()
        .path("/body");

    assert!(req.matches(&p.and(concat)));
}

#[test]
fn json() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json::<Vec<i32>>();

    let req = warp::test::request()
        .body("[1, 2, 3]");

    let vec = req.filter(&json).unwrap();
    assert_eq!(vec, &[1, 2, 3]);


    let req = warp::test::request()
        .header("content-type", "application/json")
        .body("[3, 2, 1]");

    let vec = req.filter(&json).unwrap();
    assert_eq!(vec, &[3, 2, 1], "matches content-type");
}

#[test]
fn json_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json::<Vec<i32>>()
        .map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("[3, 2, 1]");

    let res = req.reply(&json);
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[test]
fn form() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>();

    let req = warp::test::request()
        .body("foo=bar&baz=quux");

    let vec = req.filter(&form).unwrap();
    let expected = vec![
        ("foo".to_owned(), "bar".to_owned()),
        ("baz".to_owned(), "quux".to_owned()),
    ];
    assert_eq!(vec, expected);
}

#[test]
fn form_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>()
        .map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "application/x-www-form-urlencoded")
        .body("foo=bar");

    let res = req.reply(&form);
    assert_eq!(res.status(), 200);


    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("foo=bar");
    let res = req.reply(&form);
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

