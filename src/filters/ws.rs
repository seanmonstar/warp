//! Websockets Filters

use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::header;
use crate::filter::{filter_fn_one, Filter, One};
use crate::reject::Rejection;
use crate::reply::{Reply, Response};
use futures::{future, ready, FutureExt, Sink, Stream, TryFutureExt};
use headers::{Connection, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, Upgrade};
use http;
use hyper::upgrade::OnUpgrade;
use tokio_tungstenite::{
    tungstenite::protocol::{self, WebSocketConfig},
    WebSocketStream,
};

/// Creates a Websocket Filter.
///
/// The yielded `Ws` is used to finish the websocket upgrade.
///
/// # Note
///
/// This filter combines multiple filters internally, so you don't need them:
///
/// - Method must be `GET`
/// - Header `connection` must be `upgrade`
/// - Header `upgrade` must be `websocket`
/// - Header `sec-websocket-version` must be `13`
/// - Header `sec-websocket-key` must be set.
///
/// If the filters are met, yields a `Ws`. Calling `Ws::on_upgrade` will
/// return a reply with:
///
/// - Status of `101 Switching Protocols`
/// - Header `connection: upgrade`
/// - Header `upgrade: websocket`
/// - Header `sec-websocket-accept` with the hash value of the received key.
pub fn ws() -> impl Filter<Extract = One<Ws>, Error = Rejection> + Copy {
    let connection_has_upgrade = header::header2()
        .and_then(|conn: ::headers::Connection| {
            if conn.contains("upgrade") {
                future::ok(())
            } else {
                future::err(crate::reject::known(MissingConnectionUpgrade))
            }
        })
        .untuple_one();

    crate::get()
        .and(connection_has_upgrade)
        .and(header::exact_ignore_case("upgrade", "websocket"))
        .and(header::exact("sec-websocket-version", "13"))
        //.and(header::exact2(Upgrade::websocket()))
        //.and(header::exact2(SecWebsocketVersion::V13))
        .and(header::header2::<SecWebsocketKey>())
        .and(on_upgrade())
        .map(
            move |key: SecWebsocketKey, on_upgrade: Option<OnUpgrade>| Ws {
                config: None,
                key,
                on_upgrade,
            },
        )
}

/// Extracted by the [`ws`](ws) filter, and used to finish an upgrade.
pub struct Ws {
    config: Option<WebSocketConfig>,
    key: SecWebsocketKey,
    on_upgrade: Option<OnUpgrade>,
}

impl Ws {
    /// Finish the upgrade, passing a function to handle the `WebSocket`.
    ///
    /// The passed function must return a `Future`.
    pub fn on_upgrade<F, U>(self, func: F) -> impl Reply
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Output = ()> + Send + 'static,
    {
        WsReply {
            ws: self,
            on_upgrade: func,
        }
    }

    // config

    /// Set the size of the internal message send queue.
    pub fn max_send_queue(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_send_queue = Some(max);
        self
    }

    /// Set the maximum message size (defaults to 64 megabytes)
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_message_size = Some(max);
        self
    }

    /// Set the maximum frame size (defaults to 16 megabytes)
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_frame_size = Some(max);
        self
    }
}

impl fmt::Debug for Ws {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ws").finish()
    }
}

#[allow(missing_debug_implementations)]
struct WsReply<F> {
    ws: Ws,
    on_upgrade: F,
}

impl<F, U> Reply for WsReply<F>
where
    F: FnOnce(WebSocket) -> U + Send + 'static,
    U: Future<Output = ()> + Send + 'static,
{
    fn into_response(self) -> Response {
        if let Some(on_upgrade) = self.ws.on_upgrade {
            let on_upgrade_cb = self.on_upgrade;
            let config = self.ws.config;
            let fut = on_upgrade
                .and_then(move |upgraded| {
                    tracing::trace!("websocket upgrade complete");
                    WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config).map(Ok)
                })
                .and_then(move |socket| on_upgrade_cb(socket).map(Ok))
                .map(|result| {
                    if let Err(err) = result {
                        tracing::debug!("ws upgrade error: {}", err);
                    }
                });
            ::tokio::task::spawn(fut);
        } else {
            tracing::debug!("ws couldn't be upgraded since no upgrade state was present");
        }

        let mut res = http::Response::default();

        *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;

        res.headers_mut().typed_insert(Connection::upgrade());
        res.headers_mut().typed_insert(Upgrade::websocket());
        res.headers_mut()
            .typed_insert(SecWebsocketAccept::from(self.ws.key));

        res
    }
}

// Extracts OnUpgrade state from the route.
fn on_upgrade() -> impl Filter<Extract = (Option<OnUpgrade>,), Error = Rejection> + Copy {
    filter_fn_one(|route| future::ready(Ok(route.extensions_mut().remove::<OnUpgrade>())))
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
///
/// Ping messages sent from the client will be handled internally by replying with a Pong message.
/// Close messages need to be handled explicitly: usually by closing the `Sink` end of the
/// `WebSocket`.
///
/// **Note!**
/// Due to rust futures nature, pings won't be handled until read part of `WebSocket` is polled

pub struct WebSocket {
    inner: WebSocketStream<hyper::upgrade::Upgraded>,
}

impl WebSocket {
    pub(crate) async fn from_raw_socket(
        upgraded: hyper::upgrade::Upgraded,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        WebSocketStream::from_raw_socket(upgraded, role, config)
            .map(|inner| WebSocket { inner })
            .await
    }

    /// Gracefully close this websocket.
    pub async fn close(mut self) -> Result<(), crate::Error> {
        future::poll_fn(|cx| Pin::new(&mut self).poll_close(cx)).await
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Message::from_protocol(item)))),
            Some(Err(e)) => {
                tracing::debug!("websocket poll error: {}", e);
                Poll::Ready(Some(Err(crate::Error::new(e))))
            }
            None => {
                tracing::trace!("websocket closed");
                Poll::Ready(None)
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = crate::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_ready(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(crate::Error::new(e))),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match Pin::new(&mut self.inner).start_send(item.into_protocol()) {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::debug!("websocket start_send error: {}", e);
                Err(crate::Error::new(e))
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_flush(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(crate::Error::new(e))),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_close(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(err) => {
                tracing::debug!("websocket close error: {}", err);
                Poll::Ready(Err(crate::Error::new(err)))
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WebSocket").finish()
    }
}

/// Represents a WebSocket message.
#[non_exhaustive]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Message {
    /// Regular string message.
    Text(String),
    /// Regular binary message.
    Binary(Vec<u8>),
    /// Ping message, used to check latency, typically responded to with a pong message.
    Ping(Vec<u8>),
    /// Pong message, used to check latency, typically sent in response to a ping message.
    Pong(Vec<u8>),
    /// Regular closure of the websocket, with a closure code and reason.
    Close(u16, Cow<'static, str>),
    /// An abrupt unexpected closure of the websocket.
    AbruptClose,
}

impl Message {
    fn from_protocol(m: protocol::Message) -> Message {
        match m {
            protocol::Message::Text(s) => Message::Text(s),
            protocol::Message::Binary(v) => Message::Binary(v),
            protocol::Message::Ping(v) => Message::Ping(v),
            protocol::Message::Pong(v) => Message::Pong(v),
            protocol::Message::Close(Some(c)) => Message::Close(c.code.into(), c.reason),
            protocol::Message::Close(None) => Message::AbruptClose,
        }
    }

    fn into_protocol(self) -> protocol::Message {
        match self {
            Message::Text(s) => protocol::Message::Text(s),
            Message::Binary(v) => protocol::Message::Binary(v),
            Message::Ping(v) => protocol::Message::Ping(v),
            Message::Pong(v) => protocol::Message::Pong(v),
            Message::Close(code, reason) => protocol::Message::Close(Some(protocol::CloseFrame {
                code: code.into(),
                reason,
            })),
            Message::AbruptClose => protocol::Message::Close(None),
        }
    }

    /// Construct a new Text `Message`.
    pub fn text<S: Into<String>>(s: S) -> Message {
        Message::Text(s.into())
    }

    /// Construct a new Binary `Message`.
    pub fn binary<V: Into<Vec<u8>>>(v: V) -> Message {
        Message::Binary(v.into())
    }

    /// Construct a new Ping `Message`.
    pub fn ping<V: Into<Vec<u8>>>(v: V) -> Message {
        Message::Ping(v.into())
    }

    /// Construct a new Pong `Message`.
    ///
    /// Note that one rarely needs to manually construct a Pong message because the underlying tungstenite socket
    /// automatically responds to the Ping messages it receives. Manual construction might still be useful in some cases
    /// like in tests or to send unidirectional heartbeats.
    pub fn pong<V: Into<Vec<u8>>>(v: V) -> Message {
        Message::Pong(v.into())
    }

    /// Construct the default Close `Message`.
    pub fn close() -> Message {
        Message::AbruptClose
    }

    /// Construct a Close `Message` with a code and reason.
    pub fn close_with(code: impl Into<u16>, reason: impl Into<Cow<'static, str>>) -> Message {
        Message::Close(code.into(), reason.into())
    }

    /// Returns true if this message is a Text message.
    pub fn is_text(&self) -> bool {
        matches!(self, Message::Text(_))
    }

    /// Returns true if this message is a Binary message.
    pub fn is_binary(&self) -> bool {
        matches!(self, Message::Binary(_))
    }

    /// Returns true if this message a is a Close message.
    pub fn is_close(&self) -> bool {
        matches!(self, Message::Close(_, _)) || matches!(self, Message::AbruptClose)
    }

    /// Returns true if this message is a Ping message.
    pub fn is_ping(&self) -> bool {
        matches!(self, Message::Ping(_))
    }

    /// Returns true if this message is a Pong message.
    pub fn is_pong(&self) -> bool {
        matches!(self, Message::Pong(_))
    }

    /// Try to get the close frame (close code and reason)
    pub fn close_frame(&self) -> Option<(u16, &str)> {
        match self {
            Message::Close(code, reason) => Some((*code, reason.as_ref())),
            Message::AbruptClose => None,
            _ => None,
        }
    }

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self {
            Message::Text(s) => Ok(s),
            _ => Err(()),
        }
    }

    /// Return the bytes of this message, if the message can contain data.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Message::Text(s) => s.as_bytes(),
            Message::Binary(v) | Message::Ping(v) | Message::Pong(v) => v,
            _ => &[],
        }
    }

    /// Destructure this message into binary data.
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Message::Text(s) => s.into_bytes(),
            Message::Binary(v) | Message::Ping(v) | Message::Pong(v) => v,
            Message::Close(_, s) => s.into_owned().into_bytes(),
            Message::AbruptClose => Vec::new(),
        }
    }
}

impl From<Message> for Vec<u8> {
    fn from(m: Message) -> Self {
        m.into_bytes()
    }
}

// ===== Rejections =====

/// Connection header did not include 'upgrade'
#[derive(Debug)]
pub struct MissingConnectionUpgrade;

impl ::std::fmt::Display for MissingConnectionUpgrade {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Connection header did not include 'upgrade'")
    }
}

impl ::std::error::Error for MissingConnectionUpgrade {}
