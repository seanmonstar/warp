#![deny(warnings)]
use warp::{http::StatusCode, Filter};

async fn dyn_reply(word: String) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    if &word == "hello" {
        // a cast is needed for now, see https://github.com/rust-lang/rust/issues/60424
        Ok(Box::new("world") as Box<dyn warp::Reply>)
    } else {
       Ok(Box::new(StatusCode::BAD_REQUEST) as Box<dyn warp::Reply>)
    }
}

#[tokio::main]
async fn main() {
    let routes = warp::path::param().and_then(dyn_reply);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
