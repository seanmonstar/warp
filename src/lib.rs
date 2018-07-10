#![deny(warnings, missing_docs, missing_debug_implementations)]

//! warp

extern crate base64;
extern crate bytes;
extern crate crossbeam_channel;
#[macro_use] extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;
extern crate serde;
extern crate serde_json;
extern crate serde_urlencoded;
extern crate sha1;
extern crate tokio;
extern crate tokio_tungstenite;
extern crate tungstenite;

mod blocking;
mod error;
mod filter;
mod filters;
mod never;
pub mod reject;
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
    fs,
    header,
    // header() function
    header::header,
    method::{get, method, post, put, delete},
    path,
    // the index() function
    path::index,
    // path() function
    path::path,
    query,
    // query() function
    query::query,
    ws,
    // ws() function
    ws::ws,
};
pub use self::reject::{reject, Rejection};
pub use self::reply::reply;
pub use self::server::{serve, Server};
pub use hyper::rt::spawn;

pub use futures::{Future, Stream};

pub(crate) type Request = http::Request<hyper::Body>;
