#![deny(warnings)]
use warp::Filter;

#[tokio::main]
async fn main() {
    // Match foo/bar and foo/bar/  - mandatory trailing slash
    // Match foo/bar_2 and foo/bar_2/  - not mandatory trailing slash
    //
    // in the first route  trailing slash is mandatory
    // if there is no trailing slash, returns reject redirect to same uri with trailing slash
    // in the second route the trailing slash is not mandatory
    let routes = warp::path!("foo" / "bar")
        .and(warp::path::trailing_slash_or_redirect())
        .map(|| "foo bar")
        .or(warp::path!("foo" / "bar_2").map(|| "foo bar_2"));

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
