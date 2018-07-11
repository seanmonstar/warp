//! Reply Filters

use http::header::{HeaderName, HeaderValue};
use http::HttpTryFrom;

use ::filter::{Filter, Cons};
use ::reply::{Reply, Reply_};

/// Wrap a [`Filter`](::Filter) that adds a header to the reply.
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

/// dox
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

// TODO: could maybe implement Func, to allow `inner.map(with_header)`?
/// dox
#[derive(Clone, Debug)]
pub struct WithHeader {
    name: HeaderName,
    value: HeaderValue,
}

impl WithHeader {
    /// dox
    pub fn decorate<F, R>(&self, inner: F) -> impl Filter<Extract=Cons<Reply_>, Error=F::Error>
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

/// dox
#[derive(Clone, Debug)]
pub struct WithDefaultHeader {
    name: HeaderName,
    value: HeaderValue,
}

impl WithDefaultHeader {
    /// dox
    pub fn decorate<F, R>(&self, inner: F) -> impl Filter<Extract=Cons<Reply_>, Error=F::Error>
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

