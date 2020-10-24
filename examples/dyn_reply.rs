#![deny(warnings)]

use serde_derive::{Deserialize, Serialize};
use warp::{http::StatusCode, Filter};

#[derive(Deserialize, Serialize, Clone)]
struct Value {
    value: String,
}

async fn dyn_reply(word: String) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match word.as_str() {
        "hello" => Ok(Box::new("world")), // how to reply "world" with different status code
        "world" => Ok(Box::new(warp::reply::json(&Value {
            value: "Good".to_string(),
        }))), // how to reply json with different status code
        _ => Ok(Box::new(StatusCode::BAD_REQUEST)),
    }
}

#[tokio::main]
async fn main() {
    let routes = warp::path::param().and_then(dyn_reply);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
