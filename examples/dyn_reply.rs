#![deny(warnings)]

use serde_derive::{Deserialize, Serialize};
use warp::{http::Response, http::StatusCode, Filter, Reply};

#[derive(Deserialize, Serialize, Clone)]
struct Value {
    value: String,
}

struct MyReply {
    msg: String,
    value: usize,
}

impl Reply for MyReply {
    fn into_response(self) -> warp::reply::Response {
        Response::new(format!("message: {}, value: {}", self.msg, self.value).into())
    }
}

async fn dyn_reply(word: String) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    match word.as_str() {
        "hello" => Ok(Box::new("world")),
        "json" => Ok(Box::new(warp::reply::json(&Value {
            value: "Json Okay".to_string(),
        }))),
        "json201" => Ok(Box::new(
            warp::reply::json(&Value {
                value: "Json create".to_string(),
            })
            .with_status(StatusCode::CREATED),
        )),
        "response" => Ok(Box::new(
            Response::builder()
                .status(201)
                .body("Response created")
                .unwrap(),
        )),
        "myreply" => Ok(Box::new(
            MyReply {
                msg: "My own reply type working".to_string(),
                value: 42,
            }
            .with_status(StatusCode::ACCEPTED),
        )),
        _ => Ok(Box::new(StatusCode::BAD_REQUEST)),
    }
}

/// Demo dynamic reply:
/// Try it with `curl -i localhost:3000/<param>`
#[tokio::main]
async fn main() {
    let routes = warp::path::param().and_then(dyn_reply);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
