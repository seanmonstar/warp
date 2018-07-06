use futures::future;

use ::never::Never;
use ::filter::{Filter, filter_fn};

/// A filter that matches any route.
pub fn any() -> impl Filter<Extract=(), Error=Never> + Copy {
    filter_fn(|_| future::poll_fn(|| Ok(().into())))
}

