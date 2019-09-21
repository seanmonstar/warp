extern crate warp;

use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// Option 1: BoxedFilter
// Note that this may be useful for shortening compile times when you are composing many filters.
// Boxing the filters will use dynamic dispatch and speed up compilation while
// making it slightly slower at runtime.
pub fn assets_filter() -> BoxedFilter<(impl Reply,)> {
    warp::path("assets").and(warp::fs::dir("./assets")).boxed()
}

// Option 2: impl Filter + Clone
pub fn index_filter() -> impl Filter<Extract = (&'static str,), Error = Rejection> + Clone {
    warp::path::end().map(|| "Index page")
}

// Option 3: box only for debug builds
// In debug builds, this will return a BoxedFilter like Option 1
// but in release builds it will return the filter as in Option 2.
pub fn other_filter() -> impl Filter<Extract = (&'static str,), Error = Rejection> + Clone {
    warp::path::end().map(|| "Other page").debug_boxed()
}

pub fn main() {
    let routes = index_filter().or(assets_filter());
    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
