#![deny(warnings)]
extern crate futures;
extern crate pretty_env_logger;
extern crate warp;

use warp::{Filter, Future, Stream};

#[test]
fn upgrade() {
    let _ = pretty_env_logger::try_init();

    let route = warp::ws2().map(|ws: warp::ws::Ws2| ws.on_upgrade(|_| futures::future::ok(())));

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

#[test]
fn fail() {
    let _ = pretty_env_logger::try_init();

    let route = warp::any().map(warp::reply);

    warp::test::ws()
        .handshake(route)
        .expect_err("handshake non-websocket route should fail");
}

#[test]
fn text() {
    let _ = pretty_env_logger::try_init();

    let mut client = warp::test::ws().handshake(ws_echo()).expect("handshake");

    client.send_text("hello warp");
    let msg = client.recv().expect("recv");
    assert_eq!(msg.to_str(), Ok("hello warp"));
}

#[test]
fn binary() {
    let _ = pretty_env_logger::try_init();

    let mut client = warp::test::ws().handshake(ws_echo()).expect("handshake");

    client.send(warp::ws::Message::binary(&b"bonk"[..]));
    let msg = client.recv().expect("recv");
    assert!(msg.is_binary());
    assert_eq!(msg.as_bytes(), b"bonk");
}

#[test]
fn closed() {
    let _ = pretty_env_logger::try_init();

    let route = warp::ws2().map(|ws: warp::ws::Ws2| {
        ws.on_upgrade(|websocket| {
            websocket
                .close()
                .map_err(|e| panic!("close error: {:?}", e))
        })
    });

    let mut client = warp::test::ws().handshake(route).expect("handshake");

    client.recv_closed().expect("closed");
}

fn ws_echo() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> {
    warp::ws2().map(|ws: warp::ws::Ws2| {
        ws.on_upgrade(|websocket| {
            // Just echo all messages back...
            let (tx, rx) = websocket.split();
            rx
            .take_while(|m| {
                futures::future::ok(!m.is_close())
            })
            .forward(tx)
            .map(|_| ())
            .map_err(|e| {
                panic!("websocket error: {:?}", e);
            })
        })
    })
}
