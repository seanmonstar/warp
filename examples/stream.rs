#![deny(warnings)]
use warp::Filter;
use futures_util::StreamExt;

#[tokio::main]
async fn main() {
    let routes = warp::body::stream().and(warp::method())
        .then(async |mut b: warp::body::BodyStream, m| {
            println!("{m:?}");
            while let Some(Ok(buf)) = b.next().await {
                println!("{buf:?}");
            }
            warp::reply()
        });
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
