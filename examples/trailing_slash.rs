#![deny(warnings)]

use warp::Filter;

// trailing slash on some routes are mandatory, on others are forbidden
// in case of wrong request it is then redirected to the correct one.

// after running this example server:
// clear; cargo run --example trailing_slash

// run this curl commands or open in browser:
// reply ok
// clear; curl -i http://127.0.0.1:3030/foo/slash_1/; echo
// clear; curl -i http://127.0.0.1:3030/foo/no_slash_2; echo
// clear; curl -i http://127.0.0.1:3030/foo/slash_3/; echo
// clear; curl -i http://127.0.0.1:3030/foo/no_slash_4; echo

// moved permanently redirect
// clear; curl -i http://127.0.0.1:3030/foo/slash_1; echo
// clear; curl -i http://127.0.0.1:3030/foo/no_slash_2/; echo
// clear; curl -i http://127.0.0.1:3030/foo/slash_3; echo
// clear; curl -i http://127.0.0.1:3030/foo/no_slash_4/; echo

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // the special character ! in the end of path!() means
    // that the request will be redirected if the trailing slash is missing
    // this is a recommended solution for "mandatory trailing slash"
    let route1 = warp::path!( "foo" / "slash_1" / ! ).map(|| "slash_1 ok");
    // the special symbol < in the end of path!() means
    // that the request will be redirected if the trailing slash is present
    // some website need this for historical reasons
    let route2 = warp::path!( "foo" / "no_slash_2" / < ).map(|| "no_slash_2 ok");

    let route3 = warp::path::path("foo")
        .and(warp::path::path("slash_3"))
        .and(warp::path::redirect_if_not_trailing_slash())
        .and(warp::path::end())
        .map(|| warp::reply::html("slash_3 ok"));

    let route4 = warp::path::path("foo")
        .and(warp::path::path("no_slash_4"))
        .and(warp::path::redirect_if_has_trailing_slash())
        .and(warp::path::end())
        .map(|| warp::reply::html("no_slash_4 ok"));

    let routes = route1.or(route2).or(route3).or(route4);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await
}
