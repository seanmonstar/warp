//! Test utilities to test your filters.
//!
//! [`Filter`](../trait.Filter.html)s can be easily tested without starting up an HTTP
//! server, by making use of the [`RequestBuilder`](./struct.RequestBuilder.html) in this
//! module.
//!
//! # Testing Filters
//!
//! It's easy to test filters, especially if smaller filters are used to build
//! up your full set. Consider these example filters:
//!
//! ```
//! use warp::Filter;
//!
//! fn sum() -> impl Filter<Extract = (u32,), Error = warp::Rejection> + Copy {
//!     warp::path::param()
//!         .and(warp::path::param())
//!         .map(|x: u32, y: u32| {
//!             x + y
//!         })
//! }
//!
//! fn math() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Copy {
//!     warp::post2()
//!         .and(sum())
//!         .map(|z: u32| {
//!             format!("Sum = {}", z)
//!         })
//! }
//! ```
//!
//! We can test some requests against the `sum` filter like this:
//!
//! ```
//! # use warp::Filter;
//! #[test]
//! fn test_sum() {
//! #    let sum = || warp::any().map(|| 3);
//!     let filter = sum();
//!
//!     // Execute `sum` and get the `Extract` back.
//!     let value = warp::test::request()
//!         .path("/1/2")
//!         .filter(&filter)
//!         .unwrap();
//!     assert_eq!(value, 3);
//!
//!     // Or simply test if a request matches (doesn't reject).
//!     assert!(
//!         !warp::test::request()
//!             .path("/1/-5")
//!             .matches(&filter)
//!     );
//! }
//! ```
//!
//! If the filter returns something that implements `Reply`, and thus can be
//! turned into a response sent back to the client, we can test what exact
//! response is returned. The `math` filter uses the `sum` filter, but returns
//! a `String` that can be turned into a response.
//!
//! ```
//! # use warp::Filter;
//! #[test]
//! fn test_math() {
//! #    let math = || warp::any().map(warp::reply);
//!     let filter = sum();
//!
//!     let res = warp::test::request()
//!         .path("/1/2")
//!         .reply(&filter);
//!     assert_eq!(res.status(), 405, "GET is not allowed");
//!
//!     let res = warp::test::request()
//!         .method("POST")
//!         .path("/1/2")
//!         .reply(&filter);
//!     assert_eq!(res.status(), 200);
//!     assert_eq!(res.body(), "Sum is 3");
//! }
//! ```

use std::error::Error as StdError;
use std::fmt;
use std::net::SocketAddr;
#[cfg(feature = "websocket")]
use std::thread;

use bytes::Bytes;
use futures::{
    future,
    Future, Stream,
};
#[cfg(feature = "websocket")]
use futures::{
    sync::{mpsc, oneshot},
    Sink,
};
use http::{
    header::{HeaderName, HeaderValue},
    HttpTryFrom, Response,
};
use serde::Serialize;
use serde_json;
use tokio::runtime::{Builder as RtBuilder, Runtime};

use filter::Filter;
use reject::Reject;
use reply::Reply;
use route::{self, Route};
use Request;

use self::inner::OneOrTuple;

/// Starts a new test `RequestBuilder`.
pub fn request() -> RequestBuilder {
    RequestBuilder {
        remote_addr: None,
        req: Request::default(),
    }
}

/// Starts a new test `WsBuilder`.
#[cfg(feature = "websocket")]
pub fn ws() -> WsBuilder {
    WsBuilder { req: request() }
}

/// A request builder for testing filters.
///
/// See [module documentation](::test) for an overview.
#[must_use = "RequestBuilder does nothing on its own"]
#[derive(Debug)]
pub struct RequestBuilder {
    remote_addr: Option<SocketAddr>,
    req: Request,
}

/// A Websocket builder for testing filters.
///
/// See [module documentation](::test) for an overview.
#[cfg(feature = "websocket")]
#[must_use = "WsBuilder does nothing on its own"]
#[derive(Debug)]
pub struct WsBuilder {
    req: RequestBuilder,
}

/// A test client for Websocket filters.
#[cfg(feature = "websocket")]
pub struct WsClient {
    tx: mpsc::UnboundedSender<::ws::Message>,
    rx: ::futures::stream::Wait<mpsc::UnboundedReceiver<Result<::ws::Message, ::Error>>>,
}

/// An error from Websocket filter tests.
#[derive(Debug)]
pub struct WsError {
    cause: Box<dyn StdError + Send + Sync>,
}

impl RequestBuilder {
    /// Sets the method of this builder.
    ///
    /// The default if not set is `GET`.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::request()
    ///     .method("POST");
    /// ```
    ///
    /// # Panic
    ///
    /// This panics if the passed string is not able to be parsed as a valid
    /// `Method`.
    pub fn method(mut self, method: &str) -> Self {
        *self.req.method_mut() = method.parse().expect("valid method");
        self
    }

    /// Sets the request path of this builder.
    ///
    /// The default is not set is `/`.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::request()
    ///     .path("/todos/33");
    /// ```
    ///
    /// # Panic
    ///
    /// This panics if the passed string is not able to be parsed as a valid
    /// `Uri`.
    pub fn path(mut self, p: &str) -> Self {
        let uri = p.parse().expect("test request path invalid");
        *self.req.uri_mut() = uri;
        self
    }

    /// Set a header for this request.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::request()
    ///     .header("accept", "application/json");
    /// ```
    ///
    /// # Panic
    ///
    /// This panics if the passed strings are not able to be parsed as a valid
    /// `HeaderName` and `HeaderValue`.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        let name: HeaderName = HttpTryFrom::try_from(key)
            .map_err(|_| ())
            .expect("invalid header name");
        let value = HttpTryFrom::try_from(value)
            .map_err(|_| ())
            .expect("invalid header value");
        self.req.headers_mut().insert(name, value);
        self
    }

    /// Set the bytes of this request body.
    ///
    /// Default is an empty body.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::request()
    ///     .body("foo=bar&baz=quux");
    /// ```
    pub fn body(mut self, body: impl AsRef<[u8]>) -> Self {
        let body = body.as_ref().to_vec();
        *self.req.body_mut() = body.into();
        self
    }

    /// Set the bytes of this request body by serializing a value into JSON.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::request()
    ///     .json(&true);
    /// ```
    pub fn json(mut self, val: &impl Serialize) -> Self {
        let vec = serde_json::to_vec(val).expect("json() must serialize to JSON");
        *self.req.body_mut() = vec.into();
        self
    }

    /// Tries to apply the `Filter` on this request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let param = warp::path::param::<u32>();
    ///
    /// let ex = warp::test::request()
    ///     .path("/41")
    ///     .filter(&param)
    ///     .unwrap();
    ///
    /// assert_eq!(ex, 41);
    ///
    /// assert!(
    ///     warp::test::request()
    ///         .path("/foo")
    ///         .filter(&param)
    ///         .is_err()
    /// );
    /// ```
    pub fn filter<F>(self, f: &F) -> Result<<F::Extract as OneOrTuple>::Output, F::Error>
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: OneOrTuple + Send + 'static,
        F::Error: Send + 'static,
    {
        self.apply_filter(f).map(|ex| ex.one_or_tuple())
    }

    /// Returns whether the `Filter` matches this request, or rejects it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let get = warp::get2();
    /// let post = warp::post2();
    ///
    /// assert!(
    ///     warp::test::request()
    ///         .method("GET")
    ///         .matches(&get)
    /// );
    ///
    /// assert!(
    ///     !warp::test::request()
    ///         .method("GET")
    ///         .matches(&post)
    /// );
    /// ```
    pub fn matches<F>(self, f: &F) -> bool
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: Send + 'static,
        F::Error: Send + 'static,
    {
        self.apply_filter(f).is_ok()
    }

    /// Returns `Response` provided by applying the `Filter`.
    ///
    /// This requires that the supplied `Filter` return a [`Reply`](Reply).
    pub fn reply<F>(self, f: &F) -> Response<Bytes>
    where
        F: Filter + 'static,
        F::Extract: Reply + Send,
        F::Error: Reject + Send,
    {
        // TODO: de-duplicate this and apply_filter()
        assert!(!route::is_set(), "nested test filter calls");

        let route = Route::new(self.req, self.remote_addr);
        let mut fut = route::set(&route, move || f.filter())
            .map(|rep| rep.into_response())
            .or_else(|rej| {
                debug!("rejected: {:?}", rej);
                Ok(rej.into_response())
            })
            .and_then(|res| {
                let (parts, body) = res.into_parts();
                body.concat2()
                    .map(|chunk| Response::from_parts(parts, chunk.into()))
            });
        let fut = future::poll_fn(move || route::set(&route, || fut.poll()));

        block_on(fut).expect("reply shouldn't fail")
    }

    fn apply_filter<F>(self, f: &F) -> Result<F::Extract, F::Error>
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: Send + 'static,
        F::Error: Send + 'static,
    {
        assert!(!route::is_set(), "nested test filter calls");

        let route = Route::new(self.req, self.remote_addr);
        let mut fut = route::set(&route, move || f.filter());
        let fut = future::poll_fn(move || route::set(&route, || fut.poll()));

        block_on(fut)
    }
}

#[cfg(feature = "websocket")]
impl WsBuilder {
    /// Sets the request path of this builder.
    ///
    /// The default is not set is `/`.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::ws()
    ///     .path("/chat");
    /// ```
    ///
    /// # Panic
    ///
    /// This panics if the passed string is not able to be parsed as a valid
    /// `Uri`.
    pub fn path(self, p: &str) -> Self {
        WsBuilder {
            req: self.req.path(p),
        }
    }

    /// Set a header for this request.
    ///
    /// # Example
    ///
    /// ```
    /// let req = warp::test::ws()
    ///     .header("foo", "bar");
    /// ```
    ///
    /// # Panic
    ///
    /// This panics if the passed strings are not able to be parsed as a valid
    /// `HeaderName` and `HeaderValue`.
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        WsBuilder {
            req: self.req.header(key, value),
        }
    }

    /// Execute this Websocket request against te provided filter.
    ///
    /// If the handshake succeeds, returns a `WsClient`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate futures;
    /// # extern crate warp;
    /// use futures::future;
    /// use warp::Filter;
    /// # fn main() {
    ///
    /// // Some route that accepts websockets (but drops them immediately).
    /// let route = warp::ws2()
    ///     .map(|ws: warp::ws::Ws2| {
    ///         ws.on_upgrade(|_| future::ok(()))
    ///     });
    ///
    /// let client = warp::test::ws()
    ///     .handshake(route)
    ///     .expect("handshake");
    /// # }
    /// ```
    pub fn handshake<F>(self, f: F) -> Result<WsClient, WsError>
    where
        F: Filter + Send + Sync + 'static,
        F::Extract: Reply + Send,
        F::Error: Reject + Send,
    {
        let (upgraded_tx, upgraded_rx) = oneshot::channel();
        let (wr_tx, wr_rx) = mpsc::unbounded();
        let (rd_tx, rd_rx) = mpsc::unbounded();

        let test_thread = ::std::thread::current();
        let test_name = test_thread.name().unwrap_or("<unknown>");
        thread::Builder::new()
            .name(test_name.into())
            .spawn(move || {
                use tungstenite::protocol;

                let (addr, srv) = ::serve(f).bind_ephemeral(([127, 0, 0, 1], 0));

                let srv = srv.map_err(|err| panic!("server error: {:?}", err));

                let mut req = self
                    .req
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .req;

                let uri = format!("http://{}{}", addr, req.uri().path())
                    .parse()
                    .expect("addr + path is valid URI");

                *req.uri_mut() = uri;

                let mut rt = new_rt();
                rt.spawn(srv);

                let upgrade = ::hyper::Client::builder()
                    .build(AddrConnect(addr))
                    .request(req)
                    .and_then(|res| res.into_body().on_upgrade());

                let upgraded = match rt.block_on(upgrade) {
                    Ok(up) => {
                        let _ = upgraded_tx.send(Ok(()));
                        up
                    }
                    Err(err) => {
                        let _ = upgraded_tx.send(Err(err));
                        return;
                    }
                };
                let io = protocol::WebSocket::from_raw_socket(
                    upgraded,
                    protocol::Role::Client,
                    Default::default(),
                );
                let (tx, rx) = ::ws::WebSocket::new(io).split();
                let write = wr_rx
                    .map_err(|()| {
                        unreachable!("mpsc::Receiver doesn't error");
                    })
                    .forward(tx.sink_map_err(|_| ()))
                    .map(|_| ());

                let read = rx
                    .take_while(|m| {
                        futures::future::ok(!m.is_close())
                    })
                    .then(|result| Ok(result))
                    .forward(rd_tx.sink_map_err(|_| ()))
                    .map(|_| ());

                rt.block_on(write.join(read)).expect("websocket forward");
            })
            .expect("websocket handshake thread");

        match upgraded_rx.wait() {
            Ok(Ok(())) => Ok(WsClient {
                tx: wr_tx,
                rx: rd_rx.wait(),
            }),
            Ok(Err(err)) => Err(WsError::new(err)),
            Err(_canceled) => panic!("websocket handshake thread panicked"),
        }
    }
}

#[cfg(feature = "websocket")]
impl WsClient {
    /// Send a "text" websocket message to the server.
    pub fn send_text(&mut self, text: impl Into<String>) {
        self.send(::ws::Message::text(text));
    }

    /// Send a websocket message to the server.
    pub fn send(&mut self, msg: ::ws::Message) {
        self.tx.unbounded_send(msg).unwrap();
    }

    /// Receive a websocket message from the server.
    pub fn recv(&mut self) -> Result<::filters::ws::Message, WsError> {
        self.rx
            .next()
            .map(|unbounded_result| {
                unbounded_result
                    .map(|result| result.map_err(WsError::new))
                    .unwrap_or_else(|_| {
                        unreachable!("mpsc Receiver never errors");
                    })
            })
            .unwrap_or_else(|| {
                // websocket is closed
                Err(WsError::new("closed"))
            })
    }

    /// Assert the server has closed the connection.
    pub fn recv_closed(&mut self) -> Result<(), WsError> {
        self.rx
            .next()
            .map(|unbounded_result| {
                unbounded_result.unwrap_or_else(|_| {
                    unreachable!("mpsc Receiver never errors");
                })
            })
            .map(|result| match result {
                Ok(msg) => Err(WsError::new(format!("received message: {:?}", msg))),
                Err(err) => Err(WsError::new(err)),
            })
            .unwrap_or_else(|| {
                // closed successfully
                Ok(())
            })
    }
}

#[cfg(feature = "websocket")]
impl fmt::Debug for WsClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WsClient").finish()
    }
}

// ===== impl WsError =====

#[cfg(feature = "websocket")]
impl WsError {
    fn new<E: Into<Box<dyn StdError + Send + Sync>>>(cause: E) -> Self {
        WsError {
            cause: cause.into(),
        }
    }
}

impl fmt::Display for WsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "websocket error: {}", self.cause)
    }
}

impl StdError for WsError {
    fn description(&self) -> &str {
        "websocket error"
    }
}

// ===== impl AddrConnect =====

#[cfg(feature = "websocket")]
struct AddrConnect(SocketAddr);

#[cfg(feature = "websocket")]
impl ::hyper::client::connect::Connect for AddrConnect {
    type Transport = ::tokio::net::tcp::TcpStream;
    type Error = ::std::io::Error;
    type Future = ::futures::future::Map<
        ::tokio::net::tcp::ConnectFuture,
        fn(Self::Transport) -> (Self::Transport, ::hyper::client::connect::Connected),
    >;

    fn connect(&self, _: ::hyper::client::connect::Destination) -> Self::Future {
        ::tokio::net::tcp::TcpStream::connect(&self.0)
            .map(|sock| (sock, ::hyper::client::connect::Connected::new()))
    }
}

fn new_rt() -> Runtime {
    let test_thread = ::std::thread::current();
    let test_name = test_thread.name().unwrap_or("<unknown>");
    let rt_name_prefix = format!("test {}; warp-test-runtime-", test_name);
    RtBuilder::new()
        .core_threads(1)
        .blocking_threads(1)
        .name_prefix(rt_name_prefix)
        .build()
        .expect("new rt")
}

fn block_on<F>(fut: F) -> Result<F::Item, F::Error>
where
    F: Future + Send + 'static,
    F::Item: Send + 'static,
    F::Error: Send + 'static,
{
    let mut rt = new_rt();
    rt.block_on(fut)
}

mod inner {
    pub trait OneOrTuple {
        type Output;

        fn one_or_tuple(self) -> Self::Output;
    }

    impl OneOrTuple for () {
        type Output = ();
        fn one_or_tuple(self) -> Self::Output {
            ()
        }
    }

    macro_rules! one_or_tuple {
        ($type1:ident) => {
            impl<$type1> OneOrTuple for ($type1,) {
                type Output = $type1;
                fn one_or_tuple(self) -> Self::Output {
                    self.0
                }
            }
        };
        ($type1:ident, $( $type:ident ),*) => {
            one_or_tuple!($( $type ),*);

            impl<$type1, $($type),*> OneOrTuple for ($type1, $($type),*) {
                type Output = Self;
                fn one_or_tuple(self) -> Self::Output {
                    self
                }
            }
        }
    }

    one_or_tuple! {
        T1,
        T2,
        T3,
        T4,
        T5,
        T6,
        T7,
        T8,
        T9,
        T10,
        T11,
        T12,
        T13,
        T14,
        T15,
        T16
    }
}
