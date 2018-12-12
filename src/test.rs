//! Test utilities to test your filters.
//!
//! [`Filter`](../trait.Filter.html)s can be easily tested without starting up an HTTP
//! server, by making use of the [`RequestBuilder`](./struct.RequestBuilder.html) in this
//! module.

use std::net::SocketAddr;

use bytes::Bytes;
use futures::{future, Future, Stream};
use http::{header::{HeaderName, HeaderValue}, HttpTryFrom, Response};
use serde::Serialize;
use serde_json;
use tokio::runtime::Builder as RtBuilder;

use ::filter::{Filter};
use ::reject::Reject;
use ::reply::{Reply, ReplySealed};
use ::route::{self, Route};
use ::Request;

use self::inner::OneOrTuple;

/// Starts a new test `RequestBuilder`.
pub fn request() -> RequestBuilder {
    RequestBuilder {
        remote_addr: None,
        req: Request::default(),
    }
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
        let uri = p.parse()
            .expect("test request path invalid");
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
        let name: HeaderName = HttpTryFrom::try_from(key).map_err(|_| ()).expect("invalid header name");
        let value = HttpTryFrom::try_from(value).map_err(|_| ()).expect("invalid header value");
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
        let vec = serde_json::to_vec(val)
            .expect("json() must serialize to JSON");
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
        self.apply_filter(f)
            .map(|ex| ex.one_or_tuple())
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
                body
                    .concat2()
                    .map(|chunk| {
                        Response::from_parts(parts, chunk.into())
                    })

            });
        let fut = future::poll_fn(move || {
            route::set(&route, || fut.poll())
        });

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
        let fut = future::poll_fn(move || {
            route::set(&route, || fut.poll())
        });

        block_on(fut)
    }
}

fn block_on<F>(fut: F) -> Result<F::Item, F::Error>
where
    F: Future + Send + 'static,
    F::Item: Send + 'static,
    F::Error: Send + 'static,
{
    let mut rt = RtBuilder::new()
        .core_threads(1)
        .blocking_threads(1)
        .name_prefix("warp-test-runtime-")
        .build()
        .expect("new rt");

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
