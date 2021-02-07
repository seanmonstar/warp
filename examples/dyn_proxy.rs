#![deny(warnings)]
use warp::hyper::body::{Body, Bytes};
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
    body: Bytes,
) -> Result<Response<Body>, Rejection> {
    let request = build_request(&method, &path, &headers, body);
    let client = Client::new();
    let response = client.request(request).await.unwrap();
    let response_status = response.status();
    let response_headers = response.headers().clone();
    let response_body = response.into_body();

    let mut response = Response::new(response_body);
    *response.status_mut() = response_status;
    *response.headers_mut() = response_headers;

    Ok(response)
}

fn build_request(
    method: &Method,
    path: &FullPath,
    headers: &HeaderMap,
    body: Bytes,
) -> Request<Body> {
    let uri = format!("{}/{}", PROXY_TARGET, path.as_str());

    let mut request = Request::builder().method(method.as_str()).uri(uri);

    for (key, value) in headers {
        request = request.header(key, value);
    }

    request.body(Body::from(body)).unwrap()
}

#[tokio::main]
async fn main() {
    let routes = warp::method()
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .and_then(proxy_request);

    println!("Proxy server to {} running.", PROXY_TARGET);
    println!("Example request:");
    println!("curl -i -X GET http://localhost:3030/ip");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
