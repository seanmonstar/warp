//! Reply using Server-Sent Events (SSE) stream.
//!
//! # Example
//!
//! ```
//! # extern crate futures;
//! # extern crate warp;
//!
//! use std::time::Duration;
//! use futures::stream::iter_ok;
//! use warp::{Filter, ServerSentEvent};
//!
//! let app = warp::get2().and(warp::path("push-notifications")).map(|| {
//!     let events = iter_ok::<_, ::std::io::Error>(vec![
//!         warp::sse::data("unnamed event").into_a(),
//!         (
//!             warp::sse::event("chat"),
//!             warp::sse::data("chat message"),
//!         ).into_a().into_b(),
//!         (
//!             warp::sse::id(13),
//!             warp::sse::event("chat"),
//!             warp::sse::data("other chat message\nwith next line"),
//!             warp::sse::retry(Duration::from_millis(5000)),
//!         ).into_b().into_b(),
//!     ]);
//!     warp::sse(warp::sse::keep(events, None))
//! });
//! ```
//!
//! Each field already is event which can be sent to client.
//! The events with multiple fields can be created by combining fields using tuples.
//!
//! See also [EventSource](https://developer.mozilla.org/en-US/docs/Web/API/EventSource) API.
//!
use self::sealed::{SseError, SseField, SseFormat, SseWrapper};
use filter::One;
use filters::header::{header, MissingHeader};
use futures::{Async, Future, Poll, Stream};
use http::header::{HeaderValue, CACHE_CONTROL, CONTENT_TYPE};
use hyper::Body;
use reply::{ReplySealed, Response};
use serde::Serialize;
use serde_json;
use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter, Write};
use std::str::FromStr;
use std::time::Duration;
use tokio::{clock::now, timer::Delay};
use {Filter, Rejection, Reply};

/// Server-sent event message
pub trait ServerSentEvent: SseFormat + Sized + Send + 'static {
    /// Convert to either A
    fn into_a<B>(self) -> EitherServerSentEvent<Self, B> {
        EitherServerSentEvent::A(self)
    }

    /// Convert to either B
    fn into_b<A>(self) -> EitherServerSentEvent<A, Self> {
        EitherServerSentEvent::B(self)
    }

    /// Convert to boxed
    fn boxed(self) -> BoxedServerSentEvent {
        BoxedServerSentEvent(Box::new(self))
    }
}

impl<T: SseFormat + Send + 'static> ServerSentEvent for T {}

/// Boxed server-sent event
#[allow(missing_debug_implementations)]
pub struct BoxedServerSentEvent(Box<SseFormat + Send>);

impl SseFormat for BoxedServerSentEvent {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        self.0.fmt_field(f, k)
    }
}

/// Either of two server-sent events
#[allow(missing_debug_implementations)]
pub enum EitherServerSentEvent<A, B> {
    /// Variant A
    A(A),
    /// Variant B
    B(B),
}

impl<A, B> SseFormat for EitherServerSentEvent<A, B>
where
    A: SseFormat,
    B: SseFormat,
{
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        use self::EitherServerSentEvent::*;
        match self {
            A(a) => a.fmt_field(f, k),
            B(b) => b.fmt_field(f, k),
        }
    }
}

#[allow(missing_debug_implementations)]
struct SseComment<T>(T);

/// Comment field (":<comment-text>")
pub fn comment<T>(comment: T) -> impl ServerSentEvent
where
    T: Display + Send + 'static,
{
    SseComment(comment)
}

impl<T: Display> SseFormat for SseComment<T> {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Comment = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseEvent<T>(T);

/// Event name field ("event:<event-name>")
pub fn event<T>(event: T) -> impl ServerSentEvent
where
    T: Display + Send + 'static,
{
    SseEvent(event)
}

impl<T: Display> SseFormat for SseEvent<T> {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Event = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseId<T>(T);

/// Identifier field ("id:<identifier>")
pub fn id<T>(id: T) -> impl ServerSentEvent
where
    T: Display + Send + 'static,
{
    SseId(id)
}

impl<T: Display> SseFormat for SseId<T> {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Id = k {
            k.fmt(f)?;
            self.0.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseRetry(Duration);

/// Retry timeout field ("retry:<timeout>")
pub fn retry(time: Duration) -> impl ServerSentEvent {
    SseRetry(time)
}

impl SseFormat for SseRetry {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Retry = k {
            k.fmt(f)?;

            let secs = self.0.as_secs();
            let millis = self.0.subsec_nanos() / 1_000_000;

            if secs > 0 {
                // format seconds
                secs.fmt(f)?;

                // pad milliseconds
                if millis < 10 {
                    f.write_str("00")?;
                } else if millis < 100 {
                    f.write_char('0')?;
                }
            }

            // format milliseconds
            millis.fmt(f)?;

            f.write_char('\n')?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseData<T>(T);

/// Data field(s) ("data:<content>")
///
/// The multiline content will be transferred
/// using sequential data fields, one per line.
pub fn data<T>(data: T) -> impl ServerSentEvent
where
    T: Display + Send + 'static,
{
    SseData(data)
}

impl<T: Display> SseFormat for SseData<T> {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Data = k {
            for line in self.0.to_string().split('\n') {
                k.fmt(f)?;
                line.fmt(f)?;
                f.write_char('\n')?;
            }
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct SseJson<T>(T);

/// Data field with JSON content ("data:<json-content>")
pub fn json<T>(data: T) -> impl ServerSentEvent
where
    T: Serialize + Send + 'static,
{
    SseJson(data)
}

impl<T: Serialize> SseFormat for SseJson<T> {
    fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
        if let SseField::Data = k {
            k.fmt(f)?;
            serde_json::to_string(&self.0)
                .map_err(|error| {
                    error!("sse::json error {}", error);
                    fmt::Error
                }).and_then(|data| data.fmt(f))?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}

macro_rules! tuple_fmt {
    (($($t:ident),+) => ($($i:tt),+)) => {
        impl<$($t),+> SseFormat for ($($t),+)
        where
            $($t: SseFormat,)+
        {
            fn fmt_field(&self, f: &mut Formatter, k: &SseField) -> fmt::Result {
                $(self.$i.fmt_field(f, k)?;)+
                Ok(())
            }
        }
    };
}

tuple_fmt!((A, B) => (0, 1));
tuple_fmt!((A, B, C) => (0, 1, 2));
tuple_fmt!((A, B, C, D) => (0, 1, 2, 3));
tuple_fmt!((A, B, C, D, E) => (0, 1, 2, 3, 4));
tuple_fmt!((A, B, C, D, E, F) => (0, 1, 2, 3, 4, 5));
tuple_fmt!((A, B, C, D, E, F, G) => (0, 1, 2, 3, 4, 5, 6));
tuple_fmt!((A, B, C, D, E, F, G, H) => (0, 1, 2, 3, 4, 5, 6, 7));

/// Gets the optional last event id from request.
///
/// ```
/// let app = warp::sse::last_event_id::<u32>();
///
/// // The identifier is present
/// assert_eq!(
///     warp::test::request()
///        .header("Last-Event-ID", "12")
///        .filter(&app)
///        .unwrap(),
///     Some(12)
/// );
///
/// // The identifier is missing
/// assert_eq!(
///     warp::test::request()
///        .filter(&app)
///        .unwrap(),
///     None
/// );
///
/// // The identifier is not a valid
/// assert!(
///     warp::test::request()
///        .header("Last-Event-ID", "abc")
///        .filter(&app)
///        .is_err(),
/// );
/// ```
pub fn last_event_id<T>() -> impl Filter<Extract = One<Option<T>>, Error = Rejection>
where
    T: FromStr + Send,
{
    header("Last-Event-ID")
        .map(Some)
        .or_else(|rejection: Rejection| {
            if rejection.find_cause::<MissingHeader>().is_some() {
                return Ok((None,));
            }
            Err(rejection)
        })
}

/// Server-sent events reply
///
/// This function converts stream of server events into reply.
///
/// ```
/// # extern crate futures;
/// # extern crate warp;
/// # extern crate serde;
/// # #[macro_use] extern crate serde_derive;
///
/// use std::time::Duration;
/// use futures::stream::iter_ok;
/// use warp::{Filter, ServerSentEvent};
///
/// #[derive(Serialize)]
/// struct Msg {
///     from: u32,
///     text: String,
/// }
///
/// let app = warp::get2().and(warp::path("sse")).map(|| {
///     let events = iter_ok::<_, ::std::io::Error>(vec![
///         // Unnamed event with data only
///         warp::sse::data("payload").boxed(),
///         // Named event with ID and retry timeout
///         (
///             warp::sse::data("other message\nwith next line"),
///             warp::sse::event("chat"),
///             warp::sse::id(1),
///             warp::sse::retry(Duration::from_millis(15000))
///         ).boxed(),
///         // Event with JSON data
///         (
///             warp::sse::id(2),
///             warp::sse::json(Msg {
///                 from: 2,
///                 text: "hello".into(),
///             }),
///         ).boxed(),
///     ]);
///     warp::sse(events)
/// });
///
/// let res = warp::test::request()
///     .method("GET")
///     .path("/sse")
///     .reply(&app)
///     .into_body();
///
/// assert_eq!(
///     res,
///     r#"data:payload
///
/// event:chat
/// data:other message
/// data:with next line
/// id:1
/// retry:15000
///
/// data:{"from":2,"text":"hello"}
/// id:2
///
/// "#
/// );
/// ```
pub fn sse<S>(event_stream: S) -> impl Reply
where
    S: Stream + Send + 'static,
    S::Item: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    SseReply { event_stream }
}

#[allow(missing_debug_implementations)]
struct SseReply<S> {
    event_stream: S,
}

impl<S> ReplySealed for SseReply<S>
where
    S: Stream + Send + 'static,
    S::Item: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    #[inline]
    fn into_response(self) -> Response {
        let mut res = Response::new(Body::wrap_stream(
            self.event_stream
                // FIXME: error logging
                .map_err(|error| {
                    error!("sse stream error: {}", error);
                    SseError
                }).and_then(|event| SseWrapper::format(&event)),
        ));
        {
            let headers = res.headers_mut();
            // Set appropriate content type
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
            // Disable response body caching
            headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        }
        res
    }
}

#[allow(missing_debug_implementations)]
struct SseKeepAlive<S> {
    event_stream: S,
    max_interval: Duration,
    alive_timer: Delay,
}

/// Keeps event source connection when no events sent over a some time.
///
/// Some proxy servers may drop HTTP connection after a some timeout of inactivity.
/// This function helps to prevent such behavior by sending dummy events with single
/// empty comment field (i.e. ":" only) each `keep_interval` of inactivity.
///
/// See [notes](https://www.w3.org/TR/2009/WD-eventsource-20090421/#notes).
pub fn keep<S>(
    event_stream: S,
    keep_interval: Option<Duration>,
) -> impl Stream<
    Item = impl ServerSentEvent + Send + 'static,
    Error = impl StdError + Send + Sync + 'static,
> + Send
         + 'static
where
    S: Stream + Send + 'static,
    S::Item: ServerSentEvent + Send,
    S::Error: StdError + Send + Sync + 'static,
{
    let max_interval = keep_interval.unwrap_or_else(|| Duration::from_secs(15));
    let alive_timer = Delay::new(now() + max_interval);
    SseKeepAlive {
        event_stream,
        max_interval,
        alive_timer,
    }
}

impl<S> Stream for SseKeepAlive<S>
where
    S: Stream + Send + 'static,
    S::Item: ServerSentEvent,
    S::Error: StdError + Send + Sync + 'static,
{
    type Item = EitherServerSentEvent<S::Item, SseComment<&'static str>>;
    type Error = SseError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.event_stream.poll() {
            Ok(Async::NotReady) => match self.alive_timer.poll() {
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Ok(Async::Ready(_)) => {
                    // restart timer
                    self.alive_timer.reset(now() + self.max_interval);
                    Ok(Async::Ready(Some(EitherServerSentEvent::B(SseComment("")))))
                }
                Err(error) => {
                    error!("sse::keep error: {}", error);
                    Err(SseError)
                }
            },
            Ok(Async::Ready(Some(event))) => {
                // restart timer
                self.alive_timer.reset(now() + self.max_interval);
                Ok(Async::Ready(Some(EitherServerSentEvent::A(event))))
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Err(error) => {
                error!("sse::keep error: {}", error);
                Err(SseError)
            }
        }
    }
}

mod sealed {
    use super::*;

    /// SSE error type
    #[derive(Debug)]
    pub struct SseError;

    impl Display for SseError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            write!(f, "sse error")
        }
    }

    impl StdError for SseError {
        fn description(&self) -> &str {
            "sse error"
        }
    }

    impl Display for SseField {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            use self::SseField::*;
            f.write_str(match self {
                Event => "event:",
                Id => "id:",
                Data => "data:",
                Retry => "retry:",
                Comment => ":",
            })
        }
    }

    /// SSE field kind
    #[allow(missing_debug_implementations)]
    pub enum SseField {
        /// Event name field
        Event,
        /// Event id field
        Id,
        /// Event data field
        Data,
        /// Retry timeout field
        Retry,
        /// Comment field
        Comment,
    }

    /// SSE formatter trait
    pub trait SseFormat {
        /// format message field
        fn fmt_field(&self, _f: &mut Formatter, _key: &SseField) -> fmt::Result {
            Ok(())
        }
    }

    /// SSE wrapper to help formatting messages
    #[allow(missing_debug_implementations)]
    pub struct SseWrapper<'a, T: 'a>(&'a T);

    impl<'a, T> SseWrapper<'a, T>
    where
        T: SseFormat + 'a,
    {
        pub fn format(event: &'a T) -> Result<String, SseError> {
            let mut buf = String::new();
            buf.write_fmt(format_args!("{}", SseWrapper(event)))
                .map_err(|_| SseError)?;
            buf.shrink_to_fit();
            Ok(buf)
        }
    }

    impl<'a, T> Display for SseWrapper<'a, T>
    where
        T: SseFormat,
    {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            self.0.fmt_field(f, &SseField::Comment)?;
            // The event name usually transferred before the other fields.
            self.0.fmt_field(f, &SseField::Event)?;
            // It is important that the data will be transferred before
            // the identifier to prevent possible losing events when
            // resuming connection.
            self.0.fmt_field(f, &SseField::Data)?;
            self.0.fmt_field(f, &SseField::Id)?;
            self.0.fmt_field(f, &SseField::Retry)?;
            f.write_char('\n')
        }
    }
}