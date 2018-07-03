extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    // A common prefix, /hello
    let prefix = warp::path("hello");

    // extract /:name
    let name = warp::path::param::<String>()
        .map(|name| warp::reply(format!("Hello, {}", name)));

    // or extract /:num
    let num = warp::path::param::<u32>()
        .map(|num| warp::reply(format!("Hello x {}!", num)));

    // /hello AND (/:num OR /:name)
    //
    // - /hello/:num
    // - /hello/:name
    let hello = prefix.and(num.or(name));

    let bye = warp::path("good")
        .and(warp::path("bye"))
        .and(warp::path::param::<String>())
        .map(|name| warp::reply(format!("Good bye, {}!", name)));

    // GET (hello)
    let routes = warp::get(
        hello
            .or(bye)
    );

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030));
}
