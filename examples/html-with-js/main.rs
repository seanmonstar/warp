#![deny(warnings)]
use warp::Filter;

#[tokio::main]
async fn main() {
    // expose all of the files in public/
    let public = warp::path!("examples" / "html-with-js" / "public")
        .and(warp::fs::dir("examples/html-with-js/public"));

    // GET / -> index html
    let index = warp::get()
        .and(warp::path::end())
        .and(warp::fs::file("examples/html-with-js/public/index.html"));

    let routes = index.or(public);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
