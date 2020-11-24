#![deny(warnings)]
use warp::Filter;

use std::sync::Arc;

#[derive(Clone)]
struct Answer(Arc<i32>);

impl Answer {
    fn new(data: i32) -> Self {
        Answer(Arc::new(data))
    }

    fn into_response(self) -> String {
        self.0.to_string()
    }
}

#[tokio::main]
async fn main() {
    // Match any request and return hello world!
    let filter = warp::any()
        .and(warp::ext::get::<Answer>())
        .map(Answer::into_response)
        .with(warp::ext::provide(Answer::new(42)));

    warp::serve(filter).run(([127, 0, 0, 1], 3030)).await;
}
