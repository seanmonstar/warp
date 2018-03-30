extern crate futures;
extern crate http;
extern crate hyper;
#[macro_use] extern crate log;

mod filter;
mod handler;
mod reply;
mod router;
mod server;

pub use self::filter::method::{GET, POST, PUT, DELETE};
pub use self::filter::paths::path;
pub use self::router::router;
pub use self::server::{serve, Server};

pub type Request = http::Request<self::reply::WarpBody>;
