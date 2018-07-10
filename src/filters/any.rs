use futures::{Future, Poll};

use ::never::Never;
use ::filter::{FilterBase, Filter};

/// A filter that matches any route.
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

