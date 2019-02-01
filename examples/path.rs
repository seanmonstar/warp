#![deny(warnings)]
extern crate warp;

use warp::{path, Filter};

fn main() {
    // Match a single parameter
    // e.g. `/add_3/39` returns "42"
    let one = path!("add_3" / usize).map(|num| format!("{}", num + 3));

    // Match two parameters in different segments
    // e.g. `/pythag/5/and/12` returns "13"
    let two =
        path!("pythag" / f64 / "and" / f64).map(|num0: f64, num1| format!("{}", num0.hypot(num1)));

    // Match an arbitrary number of parameters of the same type, comma-separated
    // e.g. `/upper_case/i,love,rust` returns "I, LOVE, RUST"
    let list = path!("upper_case" / [String]).map(|v: Vec<String>| {
        let upper_cased = v
            .iter()
            .map(String::as_str)
            .map(str::to_uppercase)
            .collect::<Vec<_>>()
            .join(", ");

        format!("{}", upper_cased)
    });

    let routes = one.or(two).or(list);

    warp::serve(routes).run(([127, 0, 0, 1], 3030));
}
