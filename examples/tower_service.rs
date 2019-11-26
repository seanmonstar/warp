extern crate warp;
extern crate hyper;

use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use warp::{Response, Request};
use warp::reject::Reject;
use tower_service::Service;

#[derive(Clone)]
struct TowerService;

#[derive(Debug)]
struct ServiceError;
impl Reject for ServiceError {}

impl Service<Request> for TowerService {
    type Response = Response;
    type Error = ServiceError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request) -> Self::Future {
        // Create the HTTP response
        let resp = Response::new("hello world".into());

        // Return the response as an immediate future
        Box::pin(futures::future::ok(resp))
    }
}

#[tokio::main]
async fn main() {

    warp::serve_service(TowerService).run(([127, 0, 0, 1], 3030)).await;
}
