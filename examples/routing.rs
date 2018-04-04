extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    pretty_env_logger::init();

    //let index = warp::path::index();

    // A common prefix, /hello
    let prefix = warp::path::exact("/hello");

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
    let hello = prefix.and(num.or(name));

    let bye = warp::path::exact("/good")
        .and(warp::path::exact("/bye"))
        .and(warp::path::<String>())
        //XXX: fix up this argument crap
        .map(|(((), ()), name)| format!("Good bye, {}!", name));

    // GET (hello)
    let routes = warp::get(
        //XXX: weirdo map that should go away
        hello.map(|((), out)| out)
            .or(bye)
    );

    warp::serve(routes.service())
        .run(([127, 0, 0, 1], 3030));
}
