#![deny(warnings)]
use bytes::buf::Buf;
use futures::Stream;
use futures::TryStreamExt;
use warp::hyper::body::Body;
use warp::hyper::{Client, Request};
use warp::{
    http::{method::Method, HeaderMap, Response},
    path::FullPath,
    Filter, Rejection,
};

static PROXY_TARGET: &'static str = "http://httpbin.org";

async fn proxy_request(
    method: Method,
    path: FullPath,
    headers: HeaderMap,
    body: impl Stream<Item = Result<impl Buf, warp::Error>> + Send + 'static,
) -> Result<Response<Body>, Rejection> {
    let request = build_request(method, path, headers, body);
    let client = Client::new();

    if let Ok(proxy_response) = client.request(request).await {
        let proxy_status = proxy_response.status();
        let proxy_headers = proxy_response.headers().clone();
        let proxy_body = proxy_response.into_body();

        let mut response = Response::new(proxy_body);
        *response.status_mut() = proxy_status;
        *response.headers_mut() = proxy_headers;

        Ok(response)
    } else {
        Ok(Response::builder()
            .status(503)
            .body("proxy server unavailable".into())
            .unwrap())
    }
}

fn build_request(
    method: Method,
    path: FullPath,
    headers: HeaderMap,
    body: impl Stream<Item = Result<impl Buf, warp::Error>> + Send + 'static,
) -> Request<Body> {
    let uri = format!("{}/{}", PROXY_TARGET, path.as_str());

    let body = body.map_ok(|mut buf| buf.copy_to_bytes(buf.remaining()));
    let mut request = Request::new(Body::wrap_stream(body));
    *request.method_mut() = method;
    *request.uri_mut() = uri.parse().unwrap();
    *request.headers_mut() = headers;
    request
}

#[tokio::main]
async fn main() {
    let routes = warp::method()
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::stream())
        .and_then(proxy_request);

    println!("Proxy server to {} running.", PROXY_TARGET);
    println!("Example request:");
    println!("curl -i -X GET http://localhost:3030/ip");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
