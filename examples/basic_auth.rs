//#![deny(warnings)]
use headers::{authorization::Basic, Authorization};
use warp::Filter;

#[tokio::main]
async fn main() {
    let routes = warp::auth::basic("Realm name").map(|auth_header: Authorization<Basic>| {
        // TODO: some password lookups done in here
        println!("authorization header = {:?}", auth_header);
        format!("Hello, {} you're authenticated", auth_header.0.username())
    });
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
