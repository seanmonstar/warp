extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    //let index = warp::path::index();

    // A common prefix, /hello
    let prefix = warp::path::exact("hello");

    // extract /:name
    let name = warp::path::<String>()
        .map(|name| format!("Hello, {}", name));

    // or extract /:num
    let num = warp::path::<u32>()
        .map(|num| format!("Hello x {}!", num));

    // /hello AND (/:num OR /:name)
    //
    // - /hello/:num
    // - /hello/:name
    //
    // `unit_and` toss the `()` from `prefix`, and just keeps the value
    // from the other filter. With trait specialization, it should be
    // possible to make `and` specialized over types returning `()`.
    let hello = prefix.unit_and(num.or(name));

    let bye = warp::path::exact("good")
        // With impl specialization, unit_add won't be needed.
        .unit_and(warp::path::exact("bye"))
        .unit_and(warp::path::<String>())
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
