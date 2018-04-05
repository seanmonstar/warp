extern crate crossbeam_channel;
extern crate frunk_core;
extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;

mod blocking;
mod filter;
pub mod header;
mod method;
pub mod path;
mod reply;
mod route;
mod server;
pub mod test;

pub use self::blocking::{blocking, blocking_new};
pub use self::filter::Filter;
pub use self::header::header;
pub use self::method::{get, post, put, delete};
pub use self::path::{path};
pub use self::reply::reply;
pub use self::server::{serve, Server};

pub type Request = http::Request<self::reply::WarpBody>;
