//! Test utilities to test your filters.
use http::Request;

use ::filter::Filter;
use ::reply::WarpBody;
use ::route::{self, Route};

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
    pub fn filter<F>(self, f: F) -> Option<F::Extract>
    where
        F: Filter,
    {
        assert!(!route::is_set(), "nested filter calls");
        let r = Route::new(self.req);
        route::set(&r, || {
            f.filter()
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
