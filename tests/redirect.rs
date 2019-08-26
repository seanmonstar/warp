#![deny(warnings)]
use warp::{http::Uri, Filter};

#[tokio::test]
async fn redirect_uri() {
    let over_there = warp::any().map(|| warp::redirect(Uri::from_static("/over-there")));

    let req = warp::test::request();
    let resp = req.reply(&over_there).await;

    assert_eq!(resp.status(), 301);
    assert_eq!(resp.headers()["location"], "/over-there");
}
