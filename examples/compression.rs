#![deny(warnings)]

use warp::Filter;

#[tokio::main]
async fn main() {
    let file = warp::path("todos").and(warp::fs::file("./examples/todos.rs"));
    let dir = warp::path("ws_chat").and(warp::fs::file("./examples/websockets_chat.rs"));

    let file_and_dir = warp::get()
        .and(file.or(dir))
        .with(warp::compression::gzip());

    let examples = warp::path("ex")
        .and(warp::fs::dir("./examples/"))
        .with(warp::compression::brotli());

    // GET /todos => gzip -> todos.rs
    // GET /ws_chat => gzip -> ws_chat.rs
    // GET /ex/... => deflate -> ./examples/...
    let routes = file_and_dir.or(examples).with(warp::compression::auto());

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}