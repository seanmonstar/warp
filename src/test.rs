//! Test utilities to test your filters.

use bytes::Bytes;
use futures::{future, Future, Stream};
use http::Response;
use tokio::executor::thread_pool::Builder as ThreadPoolBuilder;
use tokio::runtime::Builder as RtBuilder;

use ::filter::Filter;
use ::generic::HList;
use ::reject::Reject;
use ::reply::{Reply, ReplySealed};
use ::route::{self, Route};
use ::Request;

use self::inner::OneOrTuple;

/// Starts a new test `RequestBuilder`.
pub fn request() -> RequestBuilder {
    RequestBuilder {
        req: Request::default(),
    }
}

/// A request builder for testing filters.
#[derive(Debug)]
pub struct RequestBuilder {
    req: Request,
}

impl RequestBuilder {
    /// Sets the method of this builder.
    pub fn method(mut self, method: &str) -> Self {
        *self.req.method_mut() = method.parse().expect("valid method");
        self
    }

    /// Sets the request path of this builder.
    pub fn path(mut self, p: &str) -> Self {
        let uri = p.parse()
            .expect("test request path invalid");
        *self.req.uri_mut() = uri;
        self
    }

    /// Set a header for this request.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        let name: ::http::header::HeaderName = key.parse().expect("invalid header name");
        let value = value.parse().expect("invalid header value");
        self.req.headers_mut().insert(name, value);
        self
    }

    /// Set the bytes of this request body.
    pub fn body(mut self, body: impl AsRef<[u8]>) -> Self {
        let body = body.as_ref().to_vec();
        *self.req.body_mut() = body.into();
        self
    }

    /// Tries to apply the `Filter` on this request.
    pub fn filter<F>(self, f: F) -> Result<<<F::Extract as HList>::Tuple as OneOrTuple>::Output, F::Error>
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: HList + Send + 'static,
        F::Error: Send + 'static,
        <F::Extract as HList>::Tuple: OneOrTuple,
    {
        self.apply_filter(f)
            .map(|ex| ex.flatten().one_or_tuple())
    }

    /// Returns whether the `Filter` matches this request.
    pub fn matches<F>(self, f: F) -> bool
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: Send + 'static,
        F::Error: Send + 'static,
    {
        self.apply_filter(f).is_ok()
    }

    /// Returns whether the `Filter` matches this request.
    pub fn reply<F>(self, f: F) -> Response<Bytes>
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: Reply + Send + 'static,
        F::Error: Reject + Send + 'static,
    {
        // TODO: de-duplicate this and apply_filter()
        assert!(!route::is_set(), "nested test filter calls");

        let route = Route::new(self.req);
        let mut fut = route::set(&route, move || f.filter())
            .map(|rep| rep.into_response())
            .or_else(|rej| Ok(rej.into_response()))
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

    fn apply_filter<F>(self, f: F) -> Result<F::Extract, F::Error>
    where
        F: Filter,
        F::Future: Send + 'static,
        F::Extract: Send + 'static,
        F::Error: Send + 'static,
    {
        assert!(!route::is_set(), "nested test filter calls");

        let route = Route::new(self.req);
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
    let mut pool = ThreadPoolBuilder::new();
    pool.pool_size(1);

    let mut rt = RtBuilder::new()
        .threadpool_builder(pool)
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
