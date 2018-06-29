#![deny(warnings, missing_docs)]

//! warp

extern crate base64;
extern crate crossbeam_channel;
#[macro_use] extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;
#[macro_use] extern crate scoped_tls;
extern crate serde;
extern crate serde_json;
extern crate sha1;
extern crate tungstenite;
extern crate tokio_tungstenite;

mod blocking;
mod error;
mod filter;
mod filters;
mod never;
pub mod reply;
mod route;
mod server;
pub mod test;

pub use self::blocking::{blocking, blocking_new};
pub use self::error::Error;
pub use self::filter::Filter;
pub use self::filters::{
    // any() function
    any::any,
    body,
    cookie,
    // cookie() function
    cookie::cookie,
    header,
    path,
    // header() function
    header::header,
    method::{get, method, post, put, delete},
    // path() function
    path::path,
    // ws() function
    ws::ws,
};
pub use self::reply::reply;
pub use self::server::{serve, Server};
pub use hyper::rt::spawn;

pub use futures::{Future, Stream};

/// dox?
pub type Request = http::Request<self::reply::WarpBody>;
