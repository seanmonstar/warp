#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    let readme = warp::get2()
        .and(warp::path::end())
        .and(warp::fs::file("./README.md"));

    // dir already requires GET...
    let examples = warp::path("ex").and(warp::fs::dir("./examples/"));

    // GET / => README.md
    // GET /ex/... => ./examples/..
    let routes = readme.or(examples);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
