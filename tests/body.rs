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

    let vec = req.filter(json).unwrap();
    assert_eq!(vec, &[1, 2, 3]);
}

#[test]
fn form() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>();

    let req = warp::test::request()
        .body("foo=bar&baz=quux");

    let vec = req.filter(form).unwrap();
    let expected = vec![
        ("foo".to_owned(), "bar".to_owned()),
        ("baz".to_owned(), "quux".to_owned()),
    ];
    assert_eq!(vec, expected);
}

