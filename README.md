# warp

[![Travis Build Status](https://travis-ci.org/seanmonstar/warp.svg?branch=master)](https://travis-ci.org/seanmonstar/warp)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://img.shields.io/crates/v/warp.svg)](https://crates.io/crates/warp)
[![Released API docs](https://docs.rs/warp/badge.svg)](https://docs.rs/warp)

A super-easy, composable, web server framework for warp speeds.

The fundamental building block of `warp` is the `Filter`: they can be combined
and composed to express rich requirements on requests.

Thanks to its `Filter` system, warp provides these out of the box:

* Path routing and parameter extraction
* Header requirements and extraction
* Query string deserialization
* JSON and Form bodies
* Multipart form data
* Static Files and Directories
* Websockets
* Access logging

Since it builds on top of [hyper](https://hyper.rs), you automatically get:

- HTTP/1
- HTTP/2
- Asynchronous (using Tokio 0.1 by default)
- One of the fastest HTTP implementations
- Tested and **correct**
- Works on stable Rust

## Example

```rust
use warp::{self, path, Filter};

fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    warp::serve(hello)
        .run(([127, 0, 0, 1], 3030));
}
```

For more information you can check the [docs](https://docs.rs/warp) or the [examples](https://github.com/seanmonstar/warp/tree/master/examples).

## Shared State

One of the trickiest things of Rust web applications is handling shared state (for example a database). In Warp a `Filter` can be used to provide access to the state (it still needs to be `Arc<Mutex<...>>`). See [these docs](https://docs.rs/warp/0.1.20/warp/filters/any/fn.any.html) and [this example](https://github.com/seanmonstar/warp/blob/master/examples/todos.rs#L44-L47).


