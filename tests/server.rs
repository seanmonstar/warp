extern crate warp;

use warp::{Filter, reject::Rejection, filters::BoxedFilter, reply::Reply};

#[test]
fn serve_generic_filter() {
    let routes = warp::any().map(|| "ok");
    let _ = warp::serve(routes);
}

#[test]
fn serve_filter_impl() {
    fn routes() -> impl Filter<Extract = (&'static str,), Error = Rejection> {
        warp::index().map(|| "ok")
    }
    let _ = warp::serve(routes());
}

#[test]
fn serve_boxed_filter() {
    fn routes() -> BoxedFilter<(impl Reply,)> {
        warp::any().map(|| "ok").boxed()
    }
    let _ = warp::serve(routes());
}
