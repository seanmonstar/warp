extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    let readme = warp::fs::file("./README.md");
    let examples = warp::fs::dir("./examples/");

    let routes = warp::get(
        warp::index().and(readme)
            .or(warp::path::exact("ex").and(examples))
    );

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
