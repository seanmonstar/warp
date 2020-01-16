//! Request Extensions

use futures::future;

use crate::filter::{filter_fn_one, Filter};
use crate::reject::{self, Rejection};

/// Get a previously set extension of the current route.
///
/// If the extension doesn't exist, this rejects with a `MissingExtension`.
pub fn get<T: Clone + Send + Sync + 'static>(
) -> impl Filter<Extract = (T,), Error = Rejection> + Copy {
    filter_fn_one(|route| {
        let route = route
            .extensions()
            .get::<T>()
            .cloned()
            .ok_or_else(|| reject::known(MissingExtension { _p: () }));
        future::ready(route)
    })
}

unit_error! {
    /// An error used to reject if `get` cannot find the extension.
    pub MissingExtension: "Missing request extension"
}
