#![deny(warnings)]

use warp::Filter;

// testing if
// "in the event of a rejection will still go through the filter chain"
// I put a println!() in the redirect_if_not_trailing_slash().
// I declared the same routes 2 times.
// If it "goes through the chain" it must println!() 2 times for the same route.

// run this example web server:
// clear; cargo run --example trailing_slash2

// run this curl commands or open in browser:
// clear; curl -i http://127.0.0.1:3030/foo/slash_1/; echo
// clear; curl -i http://127.0.0.1:3030/foo/slash_2/; echo

// The result is convincing:
// Running `target/debug/examples/trailing_slash2`
// redirect_if_not_trailing_slash /foo/slash_1/
// redirect_if_not_trailing_slash /foo/slash_2/
// It does NOT repeat the println!().
// It means it does not "go through the chain".
// It stops the chain, when the redirect reply is sent.
// This is because this rejection is handled.

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let route1 = warp::path!( "foo" / "slash_1" / ! ).map(|| "slash_1 ok");
    let route2 = warp::path!( "foo" / "slash_2" / ! ).map(|| "slash_2 ok");
    let route3 = warp::path!( "foo" / "slash_1" / ! ).map(|| "slash_1 ok");
    let route4 = warp::path!( "foo" / "slash_2" / ! ).map(|| "slash_2 ok");

    let routes = route1.or(route2).or(route3).or(route4);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await
}
