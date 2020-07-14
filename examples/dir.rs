#![deny(warnings)]

// trailing slash on directories is mandatory.
// In case of wrong request, it is then redirected to the correct one.

// after running this server,
// clear; cargo run --example dir
// run this curl commands

// reply ok
// clear; curl -i http://127.0.0.1:3030/sub-dir1/; echo

// moved permanently redirect
// clear; curl -i http://127.0.0.1:3030/sub-dir1; echo

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    warp::serve(warp::fs::dir("examples/dir"))
        .run(([127, 0, 0, 1], 3030))
        .await;
}
