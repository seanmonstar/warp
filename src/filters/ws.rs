//! Websockets Filters

#![allow(deprecated)]

use std::fmt;
use std::io::ErrorKind::WouldBlock;
use std::str::FromStr;

use base64;
use futures::{ Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use http;
use http::header::HeaderValue;
use sha1::{Digest, Sha1};
use tungstenite::protocol;

use ::error::Kind;
use ::filter::{Filter, FilterClone, One};
use ::reject::{Rejection};
use ::reply::{ReplySealed, Reply, Response};
use super::{body, header};

#[doc(hidden)]
#[deprecated(note="will be replaced by ws2")]
pub fn ws<F, U>(fun: F) -> impl FilterClone<Extract=One<Ws>, Error=Rejection>
where
    F: Fn(WebSocket) -> U + Clone + Send + 'static,
    U: Future<Item=(), Error=()> + Send + 'static,
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
pub fn ws2() -> impl Filter<Extract=One<Ws2>, Error=Rejection> + Copy {
    ::get2()
        .and(header::if_value(&http::header::CONNECTION, connection_has_upgrade))
        .and(header::exact_ignore_case("upgrade", "websocket"))
        .and(header::exact("sec-websocket-version", "13"))
        .and(header::header::<Accept>("sec-websocket-key"))
        .and(body::body())
        .map(move |accept: Accept, body: ::hyper::Body| {
            Ws2 {
                accept,
                body,
            }
        })
}

#[allow(deprecated)]
fn ws_new<F1, F2>(factory: F1) -> impl FilterClone<Extract=One<Ws>, Error=Rejection>
where
    F1: Fn() -> F2 + Clone + Send + 'static,
    F2: Fn(WebSocket) + Send + 'static,
{
    ::get2()
        .and(header::if_value(&http::header::CONNECTION, connection_has_upgrade))
        .and(header::exact_ignore_case("upgrade", "websocket"))
        .and(header::exact("sec-websocket-version", "13"))
        .and(header::header::<Accept>("sec-websocket-key"))
        .and(body::body())
        .map(move |accept: Accept, body: ::hyper::Body| {
            let fun = factory();
            let fut = body.on_upgrade()
                .map(move |upgraded| {
                    trace!("websocket upgrade complete");

                    let io = protocol::WebSocket::from_raw_socket(upgraded, protocol::Role::Server, None);

                    fun(WebSocket {
                        inner: io,
                    });
                })
                .map_err(|err| debug!("ws upgrade error: {}", err));
            ::hyper::rt::spawn(fut);

            Ws {
                accept,
            }
        })
}

fn connection_has_upgrade(value: &HeaderValue) -> Option<()> {
    trace!("header connection has upgrade? value={:?}", value);

    value
        .to_str()
        .ok()
        .and_then(|s| {
            for opt in s.split(", ") {
                if opt.eq_ignore_ascii_case("upgrade") {
                    return Some(());
                }
            }
            None
        })
}

#[doc(hidden)]
#[deprecated(note="will be replaced with Ws2")]
pub struct Ws {
    accept: Accept,
}

#[allow(deprecated)]
impl ReplySealed for Ws {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(101)
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-accept", self.accept.0.as_str())
            .body(Default::default())
            .unwrap()
    }
}

#[allow(deprecated)]
impl fmt::Debug for Ws {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ws")
            .finish()
    }
}

/// Extracted by the [`ws2`](ws2) filter, and used to finish an upgrade.
pub struct Ws2 {
    accept: Accept,
    body: ::hyper::Body,
}

impl Ws2 {
    /// Finish the upgrade, passing a function to handle the `WebSocket`.
    ///
    /// The passed function must return a `Future`.
    pub fn on_upgrade<F, U>(self, func: F) -> impl Reply
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Item=(), Error=()> + Send + 'static,
    {
        WsReply {
            ws: self,
            on_upgrade: func,
        }
    }
}

impl fmt::Debug for Ws2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Ws2")
            .finish()
    }
}

#[allow(missing_debug_implementations)]
struct WsReply<F> {
    ws: Ws2,
    on_upgrade: F,
}

impl<F, U> ReplySealed for WsReply<F>
where
    F: FnOnce(WebSocket) -> U + Send + 'static,
    U: Future<Item=(), Error=()> + Send + 'static,
{
    fn into_response(self) -> Response {
        let on_upgrade = self.on_upgrade;
        let fut = self.ws.body.on_upgrade()
            .map_err(|err| debug!("ws upgrade error: {}", err))
            .and_then(move |upgraded| {
                trace!("websocket upgrade complete");

                let io = protocol::WebSocket::from_raw_socket(upgraded, protocol::Role::Server, None);

                on_upgrade(WebSocket {
                    inner: io,
                })
            });
        ::hyper::rt::spawn(fut);

        http::Response::builder()
            .status(101)
            .header("connection", "upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-accept", self.ws.accept.0.as_str())
            .body(Default::default())
            .unwrap()
    }
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
pub struct WebSocket {
    inner: protocol::WebSocket<::hyper::upgrade::Upgraded>,
}

impl Stream for WebSocket {
    type Item = Message;
    type Error = ::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let msg = match self.inner.read_message() {
                Ok(item) => item,
                Err(::tungstenite::Error::Io(ref err)) if err.kind() == WouldBlock => return Ok(Async::NotReady),
                Err(::tungstenite::Error::ConnectionClosed(frame)) => {
                    trace!("websocket closed: {:?}", frame);
                    return Ok(Async::Ready(None));
                },
                Err(e) => {
                    debug!("websocket poll error: {}", e);
                    return Err(Kind::Ws(e).into());
                }
            };

            match msg {
                msg @ protocol::Message::Text(..) |
                msg @ protocol::Message::Binary(..) => {
                    return Ok(Async::Ready(Some(Message {
                        inner: msg,
                    })));
                },
                protocol::Message::Ping(payload) => {
                    trace!("websocket client ping: {:?}", payload);
                    // tungstenite automatically responds to pings, so this
                    // branch should actually never happen...
                    debug_assert!(false, "tungstenite handles pings");
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
        match self.inner.write_message(item.inner) {
            Ok(()) => Ok(AsyncSink::Ready),
            Err(::tungstenite::Error::SendQueueFull(inner)) => {
                debug!("websocket send queue full");
                Ok(AsyncSink::NotReady(Message { inner }))
            },
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
            },
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
            },
            Err(err) => {
                debug!("websocket close error: {}", err);
                Err(Kind::Ws(err).into())
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WebSocket")
            .finish()
    }
}

/// A WebSocket message.
///
/// Only repesents Text and Binary messages.
///
/// This will likely become a `non-exhaustive` enum in the future, once that
/// language feature has stabilized.
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

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self.inner {
            protocol::Message::Text(ref s) => Ok(s),
            _ => Err(())
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

#[derive(Debug)]
struct Accept(String);

impl FromStr for Accept {
    type Err = ::never::Never;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut sha1 = Sha1::default();
        sha1.input(s.as_bytes());
        sha1.input(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        Ok(Accept(base64::encode(&sha1.result())))
    }
}
