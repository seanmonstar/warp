use futures::{Async, Future, Poll};

use super::{Filter, FilterBase, Tuple};

#[derive(Clone, Copy, Debug)]
pub struct UntupleOne<F> {
    pub(super) filter: F,
}

impl<F, T> FilterBase for UntupleOne<F>
where
    F: Filter<Extract = (T,)>,
    T: Tuple,
{
    type Extract = T;
    type Error = F::Error;
    type Future = UntupleOneFuture<F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        UntupleOneFuture {
            extract: self.filter.filter(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct UntupleOneFuture<F: Filter> {
    extract: F::Future,
}

impl<F, T> Future for UntupleOneFuture<F>
where
    F: Filter<Extract = (T,)>,
    T: Tuple,
{
    type Item = T;
    type Error = F::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (t,) = try_ready!(self.extract.poll());
        Ok(Async::Ready(t))
    }
}
