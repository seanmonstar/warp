use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::{http::Response, Filter};

#[derive(Deserialize, Serialize)]
struct MyObject {
    key1: String,
    key2: u32,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // get /example1?key=value
    let example1 = warp::get()
        .and(warp::path("example1"))
        .and(warp::query::<HashMap<String, String>>())
        .map(|p: HashMap<String, String>| match p.get("key") {
            Some(key) => Response::builder().body(format!("key = {}", key)),
            None => Response::builder().body(String::from("No param in query.")),
        });

    // get /example2?key1=value,key2=42
    let example2 = warp::get()
        .and(warp::path("example2"))
        .and(warp::query::<MyObject>())
        .map(|p: MyObject| {
            Response::builder().body(format!("key1 = {}, key2 = {}", p.key1, p.key2))
        });

    warp::serve(example1.or(example2))
        .run(([127, 0, 0, 1], 3030))
        .await
}
