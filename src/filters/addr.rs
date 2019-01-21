//! Socket Address filters.

use std::net::SocketAddr;

use filter::{filter_fn_one, Filter};
use never::Never;

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
pub fn remote() -> impl Filter<Extract = (Option<SocketAddr>,), Error = Never> + Copy {
    filter_fn_one(|route| Ok(route.remote_addr()))
}
