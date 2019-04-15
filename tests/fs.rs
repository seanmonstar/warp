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
    assert_eq!(res.headers()["content-type"], "text/x-markdown");
    assert_eq!(res.headers()["accept-ranges"], "bytes");

    assert_eq!(res.body(), &*contents);
}

#[test]
fn dir() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request().path("/todos.rs");
    let res = req.reply(&file);

    assert_eq!(res.status(), 200);

    let contents = fs::read("examples/todos.rs").expect("fs::read");
    assert_eq!(res.headers()["content-length"], contents.len().to_string());
    assert_eq!(res.headers()["content-type"], "text/x-rust");
    assert_eq!(res.headers()["accept-ranges"], "bytes");

    assert_eq!(res.body(), &*contents);
}

#[test]
fn dir_encoded() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request().path("/todos%2ers");
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

    let req = warp::test::request().path("/definitely-not-found");
    let res = req.reply(&file);

    assert_eq!(res.status(), 404);
}

#[test]
fn dir_bad_path() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request().path("/../README.md");
    let res = req.reply(&file);

    assert_eq!(res.status(), 404);
}

#[test]
fn dir_bad_encoded_path() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");

    let req = warp::test::request().path("/%2E%2e/README.md");
    let res = req.reply(&file);

    assert_eq!(res.status(), 404);
}

#[test]
fn dir_fallback_index_on_dir() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::dir("examples");
    let req = warp::test::request().path("/dir");
    let res = req.reply(&file);
    let contents = fs::read("examples/dir/index.html").expect("fs::read");
    assert_eq!(res.headers()["content-length"], contents.len().to_string());
    assert_eq!(res.status(), 200);
    let req = warp::test::request().path("/dir/");
    let res = req.reply(&file);
    assert_eq!(res.headers()["content-length"], contents.len().to_string());
    assert_eq!(res.status(), 200);
}

#[test]
fn not_modified() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::file("README.md");

    let req = warp::test::request();
    let body = fs::read("README.md").unwrap();
    let res1 = req.reply(&file);
    assert_eq!(res1.status(), 200);
    assert_eq!(res1.headers()["content-length"], body.len().to_string());

    // if-modified-since
    let res = warp::test::request()
        .header("if-modified-since", &res1.headers()["last-modified"])
        .reply(&file);
    assert_eq!(res.headers().get("content-length"), None);
    assert_eq!(res.status(), 304);
    assert_eq!(res.body(), "");

    // clearly too old
    let res = warp::test::request()
        .header("if-modified-since", "Sun, 07 Nov 1994 01:00:00 GMT")
        .reply(&file);
    assert_eq!(res.status(), 200);
    assert_eq!(res.body(), &body);
    assert_eq!(res1.headers()["content-length"], body.len().to_string());
}

#[test]
fn precondition() {
    let _ = pretty_env_logger::try_init();

    let file = warp::fs::file("README.md");

    let req = warp::test::request();
    let res1 = req.reply(&file);
    assert_eq!(res1.status(), 200);

    // if-unmodified-since
    let res = warp::test::request()
        .header("if-unmodified-since", &res1.headers()["last-modified"])
        .reply(&file);
    assert_eq!(res.status(), 200);

    // clearly too old
    let res = warp::test::request()
        .header("if-unmodified-since", "Sun, 07 Nov 1994 01:00:00 GMT")
        .reply(&file);
    assert_eq!(res.status(), 412);
    assert_eq!(res.body(), "");
}

#[test]
fn byte_ranges() {
    let _ = pretty_env_logger::try_init();

    let contents = fs::read("README.md").expect("fs::read README.md");
    let file = warp::fs::file("README.md");

    let res = warp::test::request()
        .header("range", "bytes=100-200")
        .reply(&file);
    assert_eq!(res.status(), 206);
    assert_eq!(
        res.headers()["content-range"],
        format!("bytes 100-200/{}", contents.len())
    );
    assert_eq!(res.headers()["content-length"], "101");
    assert_eq!(res.body(), &contents[100..=200]);

    // bad range
    let res = warp::test::request()
        .header("range", "bytes=100-10")
        .reply(&file);
    assert_eq!(res.status(), 416);
    assert_eq!(
        res.headers()["content-range"],
        format!("bytes */{}", contents.len())
    );
    assert_eq!(res.headers().get("content-length"), None);
    assert_eq!(res.body(), "");

    // out of range
    let res = warp::test::request()
        .header("range", "bytes=100-100000")
        .reply(&file);
    assert_eq!(res.status(), 416);
    assert_eq!(
        res.headers()["content-range"],
        format!("bytes */{}", contents.len())
    );
    assert_eq!(res.headers().get("content-length"), None);
    assert_eq!(res.body(), "");

    // if-range too old
    let res = warp::test::request()
        .header("range", "bytes=100-200")
        .header("if-range", "Sun, 07 Nov 1994 01:00:00 GMT")
        .reply(&file);
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-length"], contents.len().to_string());
    assert_eq!(res.headers().get("content-range"), None);
}
