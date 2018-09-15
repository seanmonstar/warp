use futures::{Future, Poll};

use super::{Filter, FilterBase, Tuple};
use reject::Reject;

#[derive(Clone, Copy, Debug)]
pub struct Flatten<F> {
    pub(super) filter: F,
}

impl<F, FINNER, T, E> FilterBase for Flatten<F>
where
    F: Filter<Extract = (FINNER,), Error = E>,
    FINNER: Filter<Extract = T, Error = E>,
    T: Tuple,
    E: Reject,
{
    type Extract = T;
    type Error = F::Error;
    type Future = FlattenFuture<F::Future>;
    #[inline]
    fn filter(&self) -> Self::Future {
        FlattenFuture {
            inner: self.filter.filter(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct FlattenFuture<F> {
    inner: F,
}

impl<FT, INNER, T, E> Future for FlattenFuture<FT>
where
    FT: Future<Item = (INNER,), Error = E>,
    INNER: Filter<Extract = T, Error = E>,
    T: Tuple,
    E: Reject,
{
    type Item = T;
    type Error = E;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (inner_filter,) = try_ready!(self.inner.poll());
        inner_filter.filter().poll()
    }
}
