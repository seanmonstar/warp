//! Reply Filters
//!
//! These "filters" behave a little differently than the rest. Instead of
//! being used directly on requests, these filters "decorate" other filters.
//! Consider these two equivalent examples:
//!
//! ## Just mapping a `Filter`
//!
//! ```
//! use warp::{Filter, Reply};
//!
//! let ok = warp::any().map(warp::reply);
//! let route = ok.map(|rep| {
//!     rep.with_header("server", "warp")
//! });
//! ```
//!
//! ## Decorating a `Filter`
//!
//! ```
//! use warp::Filter;
//!
//! let with_server = warp::reply::with::header("server", "warp");
//!
//! let ok = warp::any().map(warp::reply);
//! let route = with_server.decorate(ok);
//! ```
//!
//! Both of these examples end up in the same, but the "decorating" filter
//! can be cleaner in intent. Additionally, decorating allows adding in
//! conditional logic *before* the request enters the inner filter (though
//! the `with::header` decorator does not).

use http::header::{HeaderName, HeaderValue};
use http::HttpTryFrom;

use ::blocking::FnClone;
use ::filter::{Cons, Filter, Map};
use ::reply::{Reply, Reply_};

/// Wrap a [`Filter`](::Filter) that adds a header to the reply.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Always set `foo: bar` header.
/// let route = warp::reply::with::header("foo", "bar")
///     .decorate(warp:any().map(warp::reply));
/// ```
pub fn header<K, V>(name: K, value: V) -> WithHeader
where
    HeaderName: HttpTryFrom<K>,
    HeaderValue: HttpTryFrom<V>,
{
    let (name, value) = assert_name_and_value(name, value);
    WithHeader {
        name,
        value,
    }
}

// pub fn headers?

/// Wrap a [`Filter`](::Filter) that adds a header to the reply, if they
/// aren't already set.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Set `server: warp` if not already set.
/// let route = warp::reply::with::default_header("server", "warp")
///     .decorate(warp:any().map(warp::reply));
/// ```
pub fn default_header<K, V>(name: K, value: V) -> WithDefaultHeader
where
    HeaderName: HttpTryFrom<K>,
    HeaderValue: HttpTryFrom<V>,
{
    let (name, value) = assert_name_and_value(name, value);
    WithDefaultHeader {
        name,
        value,
    }
}

/// Decorate a `Filter` to always set a header.
#[derive(Clone, Debug)]
pub struct WithHeader {
    name: HeaderName,
    value: HeaderValue,
}

impl WithHeader {
    /// Decorates the `Filter`, returning a new one, that always adds the header.
    // Returns `Map` instea of `impl Filter` so that Clone from F can be
    // inheritted, instead of required or lost.
    pub fn decorate<F, R>(&self, inner: F) -> Map<F, impl FnClone<R, Reply_>>
    where
        F: Filter<Extract=Cons<R>>,
        R: Reply,
    {
        let with = self.clone();
        inner.map(move |reply: R| {
            let mut resp = reply.into_response();
            // Use "insert" to replace any set header...
            resp.headers_mut().insert(&with.name, with.value.clone());
            Reply_(resp)
        })
    }
}

/// Decorate a `Filter` to set a header if it is not already set.
#[derive(Clone, Debug)]
pub struct WithDefaultHeader {
    name: HeaderName,
    value: HeaderValue,
}

impl WithDefaultHeader {
    /// Decorates the `Filter`, returning a new one, that sets the header if not already set.
    // Returns `Map` instea of `impl Filter` so that Clone from F can be
    // inheritted, instead of required or lost.
    pub fn decorate<F, R>(&self, inner: F) -> Map<F, impl FnClone<R, Reply_>>
    where
        F: Filter<Extract=Cons<R>>,
        R: Reply,
    {
        let with = self.clone();
        inner.map(move |reply: R| {
            let mut resp = reply.into_response();
            resp
                .headers_mut()
                .entry(&with.name)
                .expect("parsed headername is always valid")
                .or_insert_with(|| with.value.clone());

            Reply_(resp)
        })
    }
}

fn assert_name_and_value<K, V>(name: K, value: V) -> (HeaderName, HeaderValue)
where
    HeaderName: HttpTryFrom<K>,
    HeaderValue: HttpTryFrom<V>,
{
    let name = <HeaderName as HttpTryFrom<K>>::try_from(name)
        .map_err(Into::into)
        .unwrap_or_else(|_| panic!("invalid header name"));

    let value = <HeaderValue as HttpTryFrom<V>>::try_from(value)
        .map_err(Into::into)
        .unwrap_or_else(|_| panic!("invalid header value"));

    (name, value)
}

