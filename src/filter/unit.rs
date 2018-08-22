use futures::{Async, Future, Poll};

use super::{Filter, FilterBase};

#[derive(Clone, Copy, Debug)]
pub struct Unit<T> {
    pub(super) filter: T,
}

impl<T> FilterBase for Unit<T>
where
    T: Filter<Extract = ((),)>,
{
    type Extract = ();
    type Error = T::Error;
    type Future = UnitFuture<T>;
    #[inline]
    fn filter(&self) -> Self::Future {
        UnitFuture {
            extract: self.filter.filter(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct UnitFuture<T: Filter> {
    extract: T::Future,
}

impl<T> Future for UnitFuture<T>
where
    T: Filter<Extract = ((),)>,
{
    type Item = ();
    type Error = T::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let ((),) = try_ready!(self.extract.poll());
        Ok(Async::Ready(()))
    }
}
