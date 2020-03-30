#![deny(warnings)]

use warp::Filter;

#[tokio::main]
async fn main() {
    let todos = warp::path("todos")
        .and(warp::fs::file("./examples/todos.rs"))
        .with(warp::compression::gzip());

    let ws_chat = warp::path("ws_chat").and(warp::fs::file("./examples/websockets_chat.rs"));

    let todos_and_chat = warp::get()
        .and(todos.or(ws_chat))
        .with(warp::compression::brotli());

    let examples = warp::path("ex")
        .and(warp::fs::dir("./examples/"))
        .with(warp::compression::auto());

    let small_file = warp::path("small").and(warp::fs::file("./examples/hello.rs"));

    // GET /todos   => brotli + gzip  -> todos.rs
    // GET /ws_chat => brotli         -> ws_chat.rs
    // GET /ex/...  => client defined -> ./examples/...
    // GET /small   => no compression -> hello.rs
    let routes = todos_and_chat.or(examples).or(small_file);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
