//! Test utilities to test your filters.
use http::Request;

use ::filter::{Filter, HList};
use ::reply::WarpBody;
use ::route::{self, Route};

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
    req: Request<WarpBody>,
}

impl RequestBuilder {
    /// Sets the request path of this builder.
    pub fn path(mut self, p: &str) -> Self {
        let uri = p.parse()
            .expect("test request path invalid");
        *self.req.uri_mut() = uri;
        self
    }

    /// Tries to apply the `Filter` on this request.
    pub fn filter<F>(self, f: F) -> Option<<<F::Extract as HList>::Tuple as OneOrTuple>::Output>
    where
        F: Filter,
        F::Extract: HList,
        <F::Extract as HList>::Tuple: OneOrTuple,
    {
        assert!(!route::is_set(), "nested filter calls");
        let r = Route::new(self.req);
        route::set(&r, || {
            f.filter()
                .map(|ex| ex.flatten().one_or_tuple())
        })
    }

    /// Returns whether the `Filter` matches this request.
    pub fn matches<F>(self, f: F) -> bool
    where
        F: Filter,
    {
        assert!(!route::is_set(), "nested filter calls");
        let r = Route::new(self.req);
        route::set(&r, || {
            f.filter().is_some()
        })
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
