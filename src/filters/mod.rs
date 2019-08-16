//! Built-in Filters
//!
//! This module mostly serves as documentation to group together the list of
//! built-in filters. Most of these are available at more convenient paths.

pub mod addr;
pub mod any;
pub mod body;
pub mod cookie;
pub mod cors;
pub mod ext;
pub mod fs;
pub mod header;
pub mod log;
pub mod method;
#[cfg(feature = "multipart")]
pub mod multipart;
pub mod path;
pub mod query;
pub mod reply;
pub mod sse;
#[cfg(feature = "websocket")]
pub mod ws;

pub use filter::BoxedFilter;
