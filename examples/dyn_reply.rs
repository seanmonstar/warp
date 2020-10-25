#![deny(warnings)]

use serde_derive::{Deserialize, Serialize};
use warp::{http::Response, http::StatusCode, Filter};

#[derive(Deserialize, Serialize, Clone)]
struct Value {
    value: String,
}

async fn dyn_reply(word: String) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match word.as_str() {
        "hello" => Ok(Box::new("world")),
        "world" => Ok(Box::new(warp::reply::json(&Value {
            value: "Good".to_string(),
        }))), // how to reply json with different status code
        "create" => Ok(Box::new(
            Response::builder()
                .status(201)
                .body("world created")
                .unwrap(),
        )),
        _ => Ok(Box::new(StatusCode::BAD_REQUEST)),
    }
}

#[tokio::main]
async fn main() {
    let routes = warp::path::param().and_then(dyn_reply);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
