use futures::TryFutureExt;
use warp::Filter;

#[tokio::main]
pub async fn main() {
    let address: std::net::SocketAddr = ([127, 0, 0, 1], 8080).into();

    let routes = warp::any()
        .and(warp::method())
        .and(warp::path::full())
        .and(
            warp::filters::query::raw()
                .or(warp::any().map(|| String::default()))
                .unify(),
        )
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(
            |method: hyper::http::Method,
             path: warp::path::FullPath,
             query_params: String,
             headers: hyper::http::HeaderMap,
             body: hyper::body::Bytes| {
                let mut full_path = path.as_str().to_string();
                if query_params != "" {
                    full_path = format!("{}?{}", full_path, query_params);
                }
                let mut hyper_request = hyper::http::Request::builder()
                    .method(method)
                    .uri(full_path)
                    .body(hyper::body::Body::from(body))
                    .expect("Request::builder() failed");
                {
                    *hyper_request.headers_mut() = headers;
                }
                hyper_request
            },
        )
        .and_then(|hyper_request: hyper::Request<hyper::Body>| {
            handler(hyper_request).map_err(|_e| warp::reject::reject())
        });

    println!("Serving at: {}", address.to_string());
    warp::serve(routes).run(address).await;
}

async fn handler(
    mut request: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, warp::Rejection> {
    // Make the client a global shareable client. This is just an example.
    let client = hyper::Client::new();

    // Manipulate the uri to your liking.
    *request.uri_mut() = "http://httpbin.org/ip"
        .parse()
        .expect("Failed to parse the uri");

    let response = client.request(request).await.expect("Request failed");
    Ok(response)
}
