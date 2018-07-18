//! Built-in Filters
//!
//! This module mostly serves as documentation to group together the list of
//! built-in filters. Most of these are available at more convenient paths.
//!
//! # Product
//!
//! You may notice that several of these filters extract some `Product` type.
//! While `Product` is currently a crate-sealed type, it can basically be
//! ignored when reading the type signature.
//!
//! If a filter extracts a `Product<String, ()>`, that simply means that it
//! extracts a `String`. If you were to `map` the filter, the argument type
//! would be exactly that, just a `String`.
//!
//! What is it? It's just some type magic that allows for automatic combining
//! and flattening of tuples. Without it, combining two filters together with
//! `and`, where one extracted `()`, and another `String`, would mean the
//! `map` would be given a single argument of `((), String,)`, which is just
//! no fun.

pub mod any;
pub mod body;
pub mod cookie;
pub mod fs;
pub mod header;
pub mod log;
pub mod method;
pub mod path;
pub mod query;
pub mod reply;
pub mod ws;
