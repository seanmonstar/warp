extern crate warp;

use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// Option 1: BoxedFilter
pub fn assets_filter() -> BoxedFilter<(impl Reply,)> {
    warp::path("assets").and(warp::fs::dir("./assets")).boxed()
}

// Option 2: impl Filter + Clone
pub fn index_filter() -> impl Filter<Extract = (&'static str,), Error = Rejection> + Clone {
    warp::path::end().map(|| "Index page")
}

pub fn main() {
    let routes = index_filter().or(assets_filter());
    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
