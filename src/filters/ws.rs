//! Websockets Filters

#![allow(deprecated)]

use std::fmt;
use std::io::ErrorKind::WouldBlock;

use futures::{future, Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use headers::{Connection, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, Upgrade};
use http;
use tungstenite::protocol::{self, WebSocketConfig};

use super::{body, header};
use error::Kind;
use filter::{Filter, FilterClone, One};
use reject::Rejection;
use reply::{Reply, Response};

#[doc(hidden)]
#[deprecated(note = "will be replaced by ws2")]
pub fn ws<F, U>(fun: F) -> impl FilterClone<Extract = One<Ws>, Error = Rejection>
where
    F: Fn(WebSocket) -> U + Clone + Send + 'static,
    U: Future<Item = (), Error = ()> + Send + 'static,
{
    ws_new(move || {
        let fun = fun.clone();
        move |sock| {
            let fut = fun(sock);
            ::hyper::rt::spawn(fut);
        }
    })
}

/// Creates a Websocket Filter.
///
/// The yielded `Ws2` is used to finish the websocket upgrade.
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
/// If the filters are met, yields a `Ws2`. Calling `Ws2::on_upgrade` will
/// return a reply with:
///
/// - Status of `101 Switching Protocols`
/// - Header `connection: upgrade`
/// - Header `upgrade: websocket`
/// - Header `sec-websocket-accept` with the hash value of the received key.
pub fn ws2() -> impl Filter<Extract = One<Ws2>, Error = Rejection> + Copy {
    let connection_has_upgrade = header::header2()
        .and_then(|conn: ::headers::Connection| {
            if conn.contains("upgrade") {
                Ok(())
            } else {
                Err(::reject::bad_request())
            }
        })
        .untuple_one();

    ::get2()
        .and(connection_has_upgrade)
        .and(header::exact_ignore_case("upgrade", "websocket"))
        .and(header::exact("sec-websocket-version", "13"))
        //.and(header::exact2(Upgrade::websocket()))
        //.and(header::exact2(SecWebsocketVersion::V13))
        .and(header::header2::<SecWebsocketKey>())
        .and(body::body())
        .map(move |key: SecWebsocketKey, body: ::hyper::Body| Ws2 {
            body,
            config: None,
            key,
        })
}

#[allow(deprecated)]
fn ws_new<F1, F2>(factory: F1) -> impl FilterClone<Extract = One<Ws>, Error = Rejection>
where
    F1: Fn() -> F2 + Clone + Send + 'static,
    F2: Fn(WebSocket) + Send + 'static,
{
    ws2().map(move |Ws2 { key, config, body }| {
        let fun = factory();
        let fut = body
            .on_upgrade()
            .map(move |upgraded| {
                trace!("websocket upgrade complete");

                let io =
                    protocol::WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config);

                fun(WebSocket { inner: io });
            })
            .map_err(|err| debug!("ws upgrade error: {}", err));
        ::hyper::rt::spawn(fut);

        Ws { key }
    })
}

#[doc(hidden)]
#[deprecated(note = "will be replaced with Ws2")]
pub struct Ws {
    key: SecWebsocketKey,
}

#[allow(deprecated)]
impl Reply for Ws {
    fn into_response(self) -> Response {
        let mut res = http::Response::default();

        *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;

        res.headers_mut().typed_insert(Connection::upgrade());
        res.headers_mut().typed_insert(Upgrade::websocket());
        res.headers_mut()
            .typed_insert(SecWebsocketAccept::from(self.key));

        res
    }
}

#[allow(deprecated)]
impl fmt::Debug for Ws {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ws").finish()
    }
}

/// Extracted by the [`ws2`](ws2) filter, and used to finish an upgrade.
pub struct Ws2 {
    body: ::hyper::Body,
    config: Option<WebSocketConfig>,
    key: SecWebsocketKey,
}

impl Ws2 {
    /// Finish the upgrade, passing a function to handle the `WebSocket`.
    ///
    /// The passed function must return a `Future`.
    pub fn on_upgrade<F, U>(self, func: F) -> impl Reply
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Item = (), Error = ()> + Send + 'static,
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
            .get_or_insert_with(|| WebSocketConfig::default())
            .max_send_queue = Some(max);
        self
    }
}

impl fmt::Debug for Ws2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ws2").finish()
    }
}

#[allow(missing_debug_implementations)]
struct WsReply<F> {
    ws: Ws2,
    on_upgrade: F,
}

impl<F, U> Reply for WsReply<F>
where
    F: FnOnce(WebSocket) -> U + Send + 'static,
    U: Future<Item = (), Error = ()> + Send + 'static,
{
    fn into_response(self) -> Response {
        let on_upgrade = self.on_upgrade;
        let config = self.ws.config;
        let fut = self
            .ws
            .body
            .on_upgrade()
            .map_err(|err| debug!("ws upgrade error: {}", err))
            .and_then(move |upgraded| {
                trace!("websocket upgrade complete");

                let io =
                    protocol::WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config);

                on_upgrade(WebSocket { inner: io })
            });
        ::hyper::rt::spawn(fut);

        let mut res = http::Response::default();

        *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;

        res.headers_mut().typed_insert(Connection::upgrade());
        res.headers_mut().typed_insert(Upgrade::websocket());
        res.headers_mut()
            .typed_insert(SecWebsocketAccept::from(self.ws.key));

        res
    }
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
pub struct WebSocket {
    inner: protocol::WebSocket<::hyper::upgrade::Upgraded>,
}

impl WebSocket {
    pub(crate) fn new(inner: protocol::WebSocket<::hyper::upgrade::Upgraded>) -> Self {
        WebSocket { inner }
    }

    /// Gracefully close this websocket.
    pub fn close(mut self) -> impl Future<Item = (), Error = ::Error> {
        future::poll_fn(move || Sink::close(&mut self))
    }
}

impl Stream for WebSocket {
    type Item = Message;
    type Error = ::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let msg = match self.inner.read_message() {
                Ok(item) => item,
                Err(::tungstenite::Error::Io(ref err)) if err.kind() == WouldBlock => {
                    return Ok(Async::NotReady);
                }
                Err(::tungstenite::Error::ConnectionClosed) => {
                    trace!("websocket closed");
                    return Ok(Async::Ready(None));
                }
                Err(e) => {
                    debug!("websocket poll error: {}", e);
                    return Err(Kind::Ws(e).into());
                }
            };

            match msg {
                msg @ protocol::Message::Text(..)
                | msg @ protocol::Message::Binary(..)
                | msg @ protocol::Message::Close(..)
                | msg @ protocol::Message::Ping(..) => {
                    return Ok(Async::Ready(Some(Message { inner: msg })));
                }
                protocol::Message::Pong(payload) => {
                    trace!("websocket client pong: {:?}", payload);
                }
            }
        }
    }
}

impl Sink for WebSocket {
    type SinkItem = Message;
    type SinkError = ::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match item.inner {
            protocol::Message::Ping(..) => {
                // warp doesn't yet expose a way to construct a `Ping` message,
                // so the only way this could is if the user is forwarding the
                // received `Ping`s straight back.
                //
                // tungstenite already auto-reponds to `Ping`s with a `Pong`,
                // so this just prevents accidentally sending extra pings.
                return Ok(AsyncSink::Ready);
            }
            _ => (),
        }

        match self.inner.write_message(item.inner) {
            Ok(()) => Ok(AsyncSink::Ready),
            Err(::tungstenite::Error::SendQueueFull(inner)) => {
                debug!("websocket send queue full");
                Ok(AsyncSink::NotReady(Message { inner }))
            }
            Err(::tungstenite::Error::Io(ref err)) if err.kind() == WouldBlock => {
                // the message was accepted and partly written, so this
                // isn't an error.
                Ok(AsyncSink::Ready)
            }
            Err(e) => {
                debug!("websocket start_send error: {}", e);
                Err(Kind::Ws(e).into())
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.inner.write_pending() {
            Ok(()) => Ok(Async::Ready(())),
            Err(::tungstenite::Error::Io(ref err)) if err.kind() == WouldBlock => {
                Ok(Async::NotReady)
            }
            Err(err) => {
                debug!("websocket poll_complete error: {}", err);
                Err(Kind::Ws(err).into())
            }
        }
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        match self.inner.close(None) {
            Ok(()) => Ok(Async::Ready(())),
            Err(::tungstenite::Error::Io(ref err)) if err.kind() == WouldBlock => {
                Ok(Async::NotReady)
            }
            Err(::tungstenite::Error::ConnectionClosed) => {
                Ok(Async::Ready(()))
            }
            Err(err) => {
                debug!("websocket close error: {}", err);
                Err(Kind::Ws(err).into())
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WebSocket").finish()
    }
}

/// A WebSocket message.
///
/// Only repesents Text and Binary messages.
///
/// This will likely become a `non-exhaustive` enum in the future, once that
/// language feature has stabilized.
#[derive(Eq, PartialEq, Clone)]
pub struct Message {
    inner: protocol::Message,
}

impl Message {
    /// Construct a new Text `Message`.
    pub fn text<S: Into<String>>(s: S) -> Message {
        Message {
            inner: protocol::Message::text(s),
        }
    }

    /// Construct a new Binary `Message`.
    pub fn binary<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::binary(v),
        }
    }

    /// Returns true if this message is a Text message.
    pub fn is_text(&self) -> bool {
        self.inner.is_text()
    }

    /// Returns true if this message is a Binary message.
    pub fn is_binary(&self) -> bool {
        self.inner.is_binary()
    }

    /// Returns true if this message a is a Close message.
    pub fn is_close(&self) -> bool {
        self.inner.is_close()
    }

    /// Returns true if this message is a Ping message.
    pub fn is_ping(&self) -> bool {
        self.inner.is_ping()
    }

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self.inner {
            protocol::Message::Text(ref s) => Ok(s),
            _ => Err(()),
        }
    }

    /// Return the bytes of this message.
    pub fn as_bytes(&self) -> &[u8] {
        match self.inner {
            protocol::Message::Text(ref s) => s.as_bytes(),
            protocol::Message::Binary(ref v) => v,
            _ => unreachable!(),
        }
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}
