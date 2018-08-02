# warp

[![Travis Build Status](https://travis-ci.org/seanmonstar/warp.svg?branch=master)](https://travis-ci.org/seanmonstar/warp)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://meritbadge.herokuapp.com/warp)](https://crates.io/crates/warp)
[![Released API docs](https://docs.rs/warp/badge.svg)](https://docs.rs/warp)

A super-easy, composable, web framework for warp speeds.

The fundamental building block of `warp` is the `Filter`: they can be combined
and composed to express rich requirements on requests.

## Example

```rust
#[macro_use]
extern crate warp;

use warp::Filter;

fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    warp::serve(hello)
        .run(([127, 0, 0, 1], 3030));
}
```
