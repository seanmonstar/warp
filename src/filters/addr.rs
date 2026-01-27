//! Socket Address filters.

use std::convert::Infallible;
use std::net::SocketAddr;

use futures_util::future;

use crate::filter::{filter_fn_one, Filter};

/// Creates a `Filter` to get the remote address of the connection.
///
/// If the underlying transport doesn't use socket addresses, this will yield
/// `None`.
///
/// # Example
///
/// ```
/// use std::net::SocketAddr;
/// use warp::Filter;
///
/// let route = warp::addr::remote()
///     .map(|addr: Option<SocketAddr>| {
///         println!("remote address = {:?}", addr);
///     });
/// ```
pub fn remote() -> impl Filter<Extract = (Option<SocketAddr>,), Error = Infallible> + Copy {
    filter_fn_one(|route| {
        future::ok(
            route
                .extensions()
                .get::<RemoteAddr>()
                .map(|RemoteAddr(addr)| *addr),
        )
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RemoteAddr(pub(crate) SocketAddr);
