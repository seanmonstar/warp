//! Test utilities to test your filters.

use futures::Future;

use ::filter::Filter;
use ::generic::HList;
use ::reject::Reject;
use ::reply::{Reply, ReplySealed};
use ::route;
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

    /// Tries to apply the `Filter` on this request.
    pub fn filter<F>(self, f: F) -> Result<<<F::Extract as HList>::Tuple as OneOrTuple>::Output, F::Error>
    where
        F: Filter,
        F::Extract: HList,
        <F::Extract as HList>::Tuple: OneOrTuple,
    {
        self.apply_filter(f)
            .map(|ex| ex.flatten().one_or_tuple())
    }

    fn apply_filter<F>(self, f: F) -> Result<F::Extract, F::Error>
    where
        F: Filter,
    {
        ::futures::future::lazy(move || {
            route::set(self.req);
            f.filter()
        })
            .wait()
    }

    /// Returns whether the `Filter` matches this request.
    pub fn matches<F>(self, f: F) -> bool
    where
        F: Filter,
    {
        self.apply_filter(f).is_ok()
    }

    /// Returns whether the `Filter` matches this request.
    pub fn reply<F>(self, f: F) -> ::reply::Response
    where
        F: Filter,
        F::Extract: Reply,
        F::Error: Reject,
    {
        self.apply_filter(f)
            .map(|r| r.into_response())
            .unwrap_or_else(|err| err.into_response())
    }
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

    impl<T1> OneOrTuple for (T1,) {
        type Output = T1;
        fn one_or_tuple(self) -> Self::Output {
            self.0
        }
    }

    impl<T1, T2> OneOrTuple for (T1, T2,) {
        type Output = Self;
        fn one_or_tuple(self) -> Self::Output {
            self
        }
    }

    impl<T1, T2, T3> OneOrTuple for (T1, T2, T3,) {
        type Output = Self;
        fn one_or_tuple(self) -> Self::Output {
            self
        }
    }

    impl<T1, T2, T3, T4> OneOrTuple for (T1, T2, T3, T4,) {
        type Output = Self;
        fn one_or_tuple(self) -> Self::Output {
            self
        }
    }
}
