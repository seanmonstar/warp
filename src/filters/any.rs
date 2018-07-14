//! A filter that matches any route.

use futures::{Future, Poll};

use ::never::Never;
use ::filter::{FilterBase, Filter};

/// A filter that matches any route.
///
/// This can be a useful building block to build new filters from,
/// since [`Filter`](::Filter) is otherwise a sealed trait.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any()
///     .map(|| {
///         "I always return this string!"
///     });
/// ```
///
/// This could allow creating a single `impl Filter` returning a specific
/// reply, that can then be used as the end of several different filter
/// chains.
pub fn any() -> impl Filter<Extract=(), Error=Never> + Copy {
    Any
}

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
struct Any;

impl FilterBase for Any {
    type Extract = ();
    type Error = Never;
    type Future = AnyFut;

    #[inline]
    fn filter(&self) -> Self::Future {
        AnyFut
    }
}

#[allow(missing_debug_implementations)]
struct AnyFut;

impl Future for AnyFut {
    type Item = ();
    type Error = Never;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(().into())
    }
}

