//! dox?
use std::str::FromStr;

use base64;
use futures::{future, Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use http;
use sha1::{Digest, Sha1};
use tungstenite::protocol;
use tokio_tungstenite::WebSocketStream;

use ::filter::{Cons, Filter};
use ::never::Never;
use ::route;
use ::reply::{Reply, Response};
use super::header;

/// Creates a Websocket Filter.
///
/// The passed function is called with each successful Websocket accepted.
pub fn ws<F, U>(fun: F) -> impl Filter<Extract=Cons<Ws>>
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

/// Creates a Websocket Filter, with a supplied factory function.
///
/// The factory function is called once for each accepted `WebSocket`. The
/// factory should return a new function that is ready to handle the
/// `WebSocket`.
pub fn ws_new<F1, F2>(factory: F1) -> impl Filter<Extract=Cons<Ws>>
where
    F1: Fn() -> F2 + Send + 'static,
    F2: Fn(WebSocket) + Send + 'static,
{
    ::get(header::exact_ignore_case("connection", "upgrade")
        .and(header::exact_ignore_case("upgrade", "websocket"))
        .and(header::exact("sec-websocket-version", "13"))
        .and(header::header::<Accept>("sec-websocket-key"))
        .map(move |accept| {
            let body = route::with(|route| {
                route.take_body()
                    .expect("ws filter needs request body")
            });
            let fun = factory();
            let fut = body.on_upgrade()
                .map(move |upgraded| {
                    trace!("websocket upgrade complete");

                    let io = WebSocketStream::from_raw_socket(upgraded, protocol::Role::Server);

                    fun(WebSocket {
                        inner: io,
                    });
                })
                .map_err(|err| debug!("ws upgrade error: {}", err));
            ::hyper::rt::spawn(fut);

            Ws {
                accept,
            }
        }))
}

/// dox?
pub struct Ws {
    accept: Accept,
}

impl Reply for Ws {
    type Future = future::FutureResult<Response, Never>;
    fn into_response(self) -> Self::Future {
        future::ok(self.into())
    }
}

impl From<Ws> for Response {
    fn from(ws: Ws) -> Response {
        http::Response::builder()
                .status(101)
                .header("content-length", "0")
                .header("connection", "upgrade")
                .header("upgrade", "websocket")
                .header("sec-websocket-accept", ws.accept.0.as_str())
                .body(Default::default())
                .unwrap()
    }
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
pub struct WebSocket {
    inner: WebSocketStream<::hyper::upgrade::Upgraded>,
}

impl Stream for WebSocket {
    type Item = Message;
    type Error = ::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            let item = try_ready!(self.inner.poll().map_err(|e| {
                debug!("websocket poll error: {}", e);
                ::Error(())
            }));

            let msg = if let Some(msg) = item {
                msg
            } else {
                return Ok(Async::Ready(None));
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
                    // Pings are just suggestions, so *try* to send a pong back,
                    // but if we're backed up, no need to do any fancy buffering
                    // or anything.
                    let _ = self.inner.start_send(protocol::Message::Pong(payload));
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
        match self.inner.start_send(item.inner) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Ok(AsyncSink::NotReady(inner)) => Ok(AsyncSink::NotReady(Message {
                inner,
            })),
            Err(e) => {
                debug!("websocket start_send error: {}", e);
                Err(::Error(()))
            }
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.poll_complete()
            .map_err(|e| {
                debug!("websocket poll_complete error: {}", e);
                ::Error(())
            })
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.inner.close()
            .map_err(|e| {
                debug!("websocket close error: {}", e);
                ::Error(())
            })
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

    /// Construct a new Text `Message`.
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

    /// Return the bytes of this message.
    pub fn as_bytes(&self) -> &[u8] {
        match self.inner {
            protocol::Message::Text(ref s) => s.as_bytes(),
            protocol::Message::Binary(ref v) => v,
            _ => unreachable!(),
        }
    }
}

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
