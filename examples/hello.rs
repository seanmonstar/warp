extern crate warp;

fn main() {
    warp::serve(|| "Hello, World!")
        .run(([127, 0, 0, 1], 3030));
}
