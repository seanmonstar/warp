extern crate crossbeam_channel;
#[macro_use] extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;

mod blocking;
mod error;
mod filter;
mod filters;
pub mod reply;
mod route;
mod server;
pub mod test;

pub use self::blocking::{blocking, blocking_new};
pub use self::error::Error;
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

pub use futures::Future;

pub type Request = http::Request<self::reply::WarpBody>;
