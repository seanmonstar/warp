extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    let readme = warp::fs::file("./README.md");

    let routes = warp::get(warp::index().and(readme));

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
