extern crate pretty_env_logger;
extern crate warp;

fn main() {
    pretty_env_logger::init();
    let index = warp::GET + "/";
    let hello_name = warp::GET + "/hello" / warp::path::<String>();
    let hello_num = warp::GET + "/hello" / warp::path::<u16>();


    warp::router()
        // These two are the same thing:
        //.route(warp::path::index(), || "Hello, World!")
        .route(index, || "Hello, World!")
        // /hello/:name
        .route(hello_name, |(), name| format!("Hello, {}", name))
        .route(hello_num, |(), num| format!("Hello x {}!", num))
        .run(([127, 0, 0, 1], 3030));
}
