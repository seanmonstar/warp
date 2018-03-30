use http::Response;
use hyper::Body;

pub trait Reply {
    fn into_response(self) -> Response<WarpBody>;
}

pub struct WarpBody(pub(crate) Body);

impl Reply for Response<WarpBody> {
    fn into_response(self) -> Response<WarpBody> {
        self
    }
}

impl Reply for &'static str {
    fn into_response(self) -> Response<WarpBody> {
        Response::builder()
            .header("content-length", &*self.len().to_string())
            .body(WarpBody(Body::from(self)))
            .unwrap()
    }
}

impl Reply for String {
    fn into_response(self) -> Response<WarpBody> {
        Response::builder()
            .header("content-length", &*self.len().to_string())
            .body(WarpBody(Body::from(self)))
            .unwrap()
    }
}
