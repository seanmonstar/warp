//! Built-in Filters
//!
//! This module mostly serves as documentation to group together the list of
//! built-in filters. Most of these are available at more convenient paths.

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

pub use filter::BoxedFilter;
