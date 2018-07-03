extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    // A common prefix, /hello
    let prefix = warp::path("hello");

    // extract /:name
    let name = warp::path::param::<String>()
        .map(|name| format!("Hello, {}", name));

    // or extract /:num
    let num = warp::path::param::<u32>()
        .map(|num| format!("Hello x {}!", num));

    // /hello AND (/:num OR /:name)
    //
    // - /hello/:num
    // - /hello/:name
    let hello = prefix.and(num.or(name));

    let bye = warp::path("good")
        .and(warp::path("bye"))
        .and(warp::path::param::<String>())
        .map(|name| format!("Good bye, {}!", name));

    // GET (hello)
    let routes = warp::get(
        hello
            .or(bye)
            // Map the strings into replies
            // With trait specialization, this won't be needed.
            .map(warp::reply)
    );

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
