# warp

A super-easy, composable, web framework for warp speeds.

The fundamental building block of `warp` is the `Filter`: they can be combined
and composed to express rich requirements on requests.

## Example

```rust
extern crate warp;

fn main() {
    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    warp::serve(hello)
        .run(([127, 0, 0, 1], 3030));
}
```
