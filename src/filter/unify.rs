use futures::{Async, Future, Poll};

use super::{Either, Filter, FilterBase, Tuple};

#[derive(Clone, Copy, Debug)]
pub struct Unify<F> {
    pub(super) filter: F,
}

impl<F, T> FilterBase for Unify<F>
where
    F: Filter<Extract = (Either<T, T>,)>,
    T: Tuple,
{
    type Extract = T;
    type Error = F::Error;
    type Future = UnifyFuture<F::Future>;
    #[inline]
    fn filter(&self) -> Self::Future {
        UnifyFuture {
            inner: self.filter.filter(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct UnifyFuture<F> {
    inner: F,
}

impl<F, T> Future for UnifyFuture<F>
where
    F: Future<Item = (Either<T, T>,)>,
{
    type Item = T;
    type Error = F::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let unified = match try_ready!(self.inner.poll()) {
            (Either::A(a),) => a,
            (Either::B(b),) => b,
        };
        Ok(Async::Ready(unified))
    }
}
