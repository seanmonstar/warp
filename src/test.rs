use http::Request;

use ::filter::Filter;
use ::reply::WarpBody;
use ::route::{self, Route};

pub fn request() -> RequestBuilder {
    RequestBuilder {
        req: Request::default(),
    }
}

#[derive(Debug)]
pub struct RequestBuilder {
    req: Request<WarpBody>,
}

impl RequestBuilder {
    pub fn path(mut self, p: &str) -> Self {
        let uri = p.parse()
            .expect("test request path invalid");
        *self.req.uri_mut() = uri;
        self
    }

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
