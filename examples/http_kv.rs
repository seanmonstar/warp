use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

use warp::{http::Response, http::StatusCode, Filter, Rejection, Reply};

// Use simple map as an in memory KV store
type Map = Arc<Mutex<HashMap<String, Value>>>;

#[derive(Deserialize, Serialize, Clone)]
struct Value {
    value: String,
}

/// Provides a RESTful web server as a simple key value store
///
/// API will be:
///
/// - `GET /:key`: get the value with key
/// - `POST /:key "{"value": "some stuff"}"`: create/update a new key with value
/// - `DELETE /:key`: delete a specific key
///
/// Testing curl command could be:
/// - `curl -i -X POST -H "Content-Type:application/json" localhost:3030/k1 -d '{"value": "v1"}'`
/// - `curl -i -X GET -H "Content-Type:application/json" localhost:3030/k1`
/// - `curl -i -X DELETE -H "Content-Type:application/json" localhost:3030/k1`
#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        // Set `RUST_LOG=todos=debug` to see debug logs,
        // this only shows access logs.
        env::set_var("RUST_LOG", "http_kv=info");
    }
    pretty_env_logger::init();
    let map: Map = Arc::new(Mutex::new(HashMap::new()));

    let map_clone = map.clone();
    let post = warp::post()
        .and(warp::path::param::<String>())
        .and(warp::body::content_length_limit(1024 * 16).and(warp::body::json()))
        .and(warp::any().map(move || map_clone.clone()))
        .and_then(|key: String, value: Value, map: Map| post_handler(key, value, map));

    let map_clone = map.clone();
    let get = warp::get()
        .and(warp::path::param::<String>())
        .and(warp::any().map(move || map_clone.clone()))
        .and_then(|key: String, map: Map| get_handler(key, map));

    let map_clone = map.clone();
    let delete = warp::delete()
        .and(warp::path::param::<String>())
        .and(warp::any().map(move || map_clone.clone()))
        .and_then(|key: String, map: Map| delete_handler(key, map));

    // View access logs by setting `RUST_LOG=todos`.
    let routes = get.or(post).or(delete).with(warp::log("http_kv"));
    // Start up the server...
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn post_handler(key: String, value: Value, map: Map) -> Result<Box<dyn Reply>, Rejection> {
    match map.lock().await.insert(key, value.clone()) {
        Some(_) => Ok(Box::new(warp::reply::json(&value))),
        None => Ok(Box::new(warp::reply::json(&value))), // TODO: return 201 here, and 200 above
    }
}

async fn get_handler(key: String, map: Map) -> Result<Box<dyn Reply>, Rejection> {
    match map.lock().await.get(key.as_str()) {
        Some(value) => Ok(Box::new(warp::reply::json(value).into_response())),
        None => Ok(Box::new(StatusCode::NOT_FOUND)),
    }
}

async fn delete_handler(key: String, map: Map) -> Result<Box<dyn Reply>, Rejection> {
    match map.lock().await.remove(key.as_str()) {
        Some(value) => Ok(Box::new(warp::reply::json(&value).into_response())),
        None => Ok(Box::new(
            Response::builder()
                .status(404)
                .body("nothing deleted")
                .unwrap(),
        )),
    }
}
