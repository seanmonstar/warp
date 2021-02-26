//#![deny(warnings)]
use headers::{authorization::Basic, Authorization};
use std::collections::HashMap;
use std::sync::Arc;
use warp::Filter;

#[tokio::main]
async fn main() {
    // insecure example, for illustration purposes only,
    // don't store passwords in cleartext in production!
    let pwdb: HashMap<_, _> = vec![
        ("alice", "wonderland"),
        ("bob", "cat"),
        ("carl", "IePai4ph"),
    ]
    .into_iter()
    .collect();
    let pwdb = Arc::new(pwdb); // no lock-guard needed, it's read-only
    let pwdb = warp::any().map(move || pwdb.clone());
    let routes = warp::auth::basic("Realm name").and(pwdb.clone()).map(
        |auth_header: Authorization<Basic>, pwdb: Arc<HashMap<&str, &str>>| {
            println!("authorization header = {:?}", auth_header);
            let user = auth_header.0.username();
            if pwdb.get(user) == Some(&auth_header.0.password()) {
                format!("Hello, {} you're authenticated", user)
            } else {
                format!("Hello, {} you've forgot your password", user)
            }
        },
    );
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
