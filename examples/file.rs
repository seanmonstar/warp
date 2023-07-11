#![deny(warnings)]

use warp::header::Conditionals;
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let readme = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("./README.md"));

    // try GET /dyn/Cargo.toml or GET /dyn/README.md
    let dynamic_file = warp::get()
        .and(warp::path::path("dyn"))
        .and(warp::path::param::<String>())
        .and(warp::header::conditionals())
        .and_then(|file_name: String, conditionals: Conditionals| {
            warp::reply::file(file_name, conditionals)
        });

    // dir already requires GET...
    let examples = warp::path("ex").and(warp::fs::dir("./examples/"));

    // GET / => README.md
    // Get /dyn/{file} => ./{file}
    // GET /ex/... => ./examples/..
    let routes = readme.or(dynamic_file).or(examples);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
