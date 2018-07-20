#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use std::fs;

#[test]
fn file() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::file("README.md");

    let req = warp::test::request();
    let res = req.reply(&file);

    assert_eq!(res.status(), 200);

    let contents = fs::read("README.md").expect("fs::read README.md");
    assert_eq!(res.headers()["content-length"], contents.len().to_string());

    assert_eq!(res.body(), &*contents);
}

#[test]
fn dir() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request()
        .path("/todos.rs");
    let res = req.reply(&file);

    assert_eq!(res.status(), 200);

    let contents = fs::read("examples/todos.rs").expect("fs::read");
    assert_eq!(res.headers()["content-length"], contents.len().to_string());

    assert_eq!(res.body(), &*contents);
}

#[test]
fn dir_not_found() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request()
        .path("/definitely-not-found");
    let res = req.reply(&file);

    assert_eq!(res.status(), 404);
}

#[test]
fn dir_bad_path() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request()
        .path("/../README.md");
    let res = req.reply(&file);

    assert_eq!(res.status(), 400);
    assert_eq!(res.body(), "");
}

