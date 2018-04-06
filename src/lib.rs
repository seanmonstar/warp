extern crate crossbeam_channel;
extern crate frunk_core;
extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;

mod blocking;
mod filter;
mod filters;
mod reply;
mod route;
mod server;
pub mod test;

pub use self::blocking::{blocking, blocking_new};
pub use self::filter::Filter;
pub use self::filters::{
    body,
    header,
    path,
    // header() function
    header::header,
    method::{get, post, put, delete},
    // path() function
    path::path,
};
pub use self::reply::reply;
pub use self::server::{serve, Server};

pub type Request = http::Request<self::reply::WarpBody>;
