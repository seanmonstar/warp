#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;
extern crate futures;

use warp::Filter;

#[test]
fn smoke() {
    let _ = pretty_env_logger::try_init();

    let route = warp::ws2()
        .map(|ws: warp::ws::Ws2| {
            ws.on_upgrade(|_| futures::future::ok(()))
        });

    // From https://tools.ietf.org/html/rfc6455#section-1.2
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let accept = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    let resp = warp::test::request()
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", key)
        .reply(&route);

    assert_eq!(resp.status(), 101);
    assert_eq!(resp.headers()["connection"], "upgrade");
    assert_eq!(resp.headers()["upgrade"], "websocket");
    assert_eq!(resp.headers()["sec-websocket-accept"], accept);

    let resp = warp::test::request()
        .header("connection", "keep-alive, Upgrade")
        .header("upgrade", "Websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", key)
        .reply(&route);

    assert_eq!(resp.status(), 101);
}
