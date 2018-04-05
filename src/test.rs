use http::Request;

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

    pub fn route(&self) -> Route {
        Route::new(&self.req)
    }
}
