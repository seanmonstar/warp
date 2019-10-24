#![deny(warnings)]

use warp::Filter;
use futures::{FutureExt, StreamExt};

#[tokio::test]
async fn upgrade() {
    let _ = pretty_env_logger::try_init();

    let route = warp::ws2().map(|ws: warp::ws::Ws2| ws.on_upgrade(|_| futures::future::ready(())));

    // From https://tools.ietf.org/html/rfc6455#section-1.2
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let accept = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    let resp = warp::test::request()
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", key)
        .reply(&route)
        .await;

    assert_eq!(resp.status(), 101);
    assert_eq!(resp.headers()["connection"], "upgrade");
    assert_eq!(resp.headers()["upgrade"], "websocket");
    assert_eq!(resp.headers()["sec-websocket-accept"], accept);

    let resp = warp::test::request()
        .header("connection", "keep-alive, Upgrade")
        .header("upgrade", "Websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", key)
        .reply(&route)
        .await;

    assert_eq!(resp.status(), 101);
}

#[tokio::test]
async fn fail() {
    let _ = pretty_env_logger::try_init();

    let route = warp::any().map(warp::reply);

    warp::test::ws()
        .handshake(route)
        .await
        .expect_err("handshake non-websocket route should fail");
}

#[tokio::test]
async fn text() {
    let _ = pretty_env_logger::try_init();

    let mut client = warp::test::ws()
        .handshake(ws_echo())
        .await
        .expect("handshake");

    client.send_text("hello warp");

    let msg = client.recv().await.expect("recv");
    assert_eq!(msg.to_str(), Ok("hello warp"));
}

#[tokio::test]
async fn binary() {
    let _ = pretty_env_logger::try_init();

    let mut client = warp::test::ws()
        .handshake(ws_echo())
        .await
        .expect("handshake");

    client.send(warp::ws::Message::binary(&b"bonk"[..]));
    let msg = client.recv().await.expect("recv");
    assert!(msg.is_binary());
    assert_eq!(msg.as_bytes(), b"bonk");
}

#[tokio::test]
async fn closed() {
    let _ = pretty_env_logger::try_init();

    let route = warp::ws2().map(|ws: warp::ws::Ws2| {
        ws.on_upgrade(|websocket| {
            websocket
                .close()
                .map(|_| ())
        })
    });

    let mut client = warp::test::ws().
        handshake(route)
        .await
        .expect("handshake");

    client.recv_closed()
        .await
        .expect("closed");
}

#[tokio::test]
async fn limit_message_size() {
    let _ = pretty_env_logger::try_init();

    let echo = warp::ws2().map(|ws: warp::ws::Ws2| {
        ws.max_message_size(1024).on_upgrade(|websocket| {
            // Just echo all messages back...
            let (tx, rx) = websocket.split();
            rx
                .forward(tx)
                .map(|result| {
                    assert!(result.is_err());
                    assert_eq!(
                        format!("{}", result.unwrap_err()).as_str(),
                        "Space limit exceeded: Message too big: 0 + 1025 > 1024"
                    );
                })
        })
    });
    let mut client = warp::test::ws()
        .handshake(echo)
        .await
        .expect("handshake");

    client.send(warp::ws::Message::binary(vec![0; 1025]));
    client.send_text("hello warp");
    assert!(client.recv().await.is_err());
}

fn ws_echo() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> {
    warp::ws2().map(|ws: warp::ws::Ws2| {
        ws.on_upgrade(|websocket| {
            // Just echo all messages back...
            let (tx, rx) = websocket.split();
            rx
                .forward(tx)
                .map(|_| ())
        })
    })
}
