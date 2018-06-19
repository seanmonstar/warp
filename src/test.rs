use http::Request;

use ::filter::Filter;
use ::reply::WarpBody;
use ::route::Route;

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

    pub fn filter<F>(mut self, f: F) -> Option<F::Extract>
    where
        F: Filter,
    {
        f.filter(Route::new(&mut self.req))
            .map(|(_, e)| e)
    }

    pub fn matches<F>(mut self, f: F) -> bool
    where
        F: Filter,
    {
        f.filter(Route::new(&mut self.req)).is_some()
    }
}
