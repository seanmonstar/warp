use futures::{Future, Poll};

use super::{Filter, FilterBase};
use reject::Reject;

#[derive(Clone, Copy, Debug)]
pub struct MapErr<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, E> FilterBase for MapErr<T, F>
where
    T: Filter,
    F: Fn(T::Error) -> E + Clone + Send,
    E: Reject,
{
    type Extract = T::Extract;
    type Error = E;
    type Future = MapErrFuture<T, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        MapErrFuture {
            extract: self.filter.filter(),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct MapErrFuture<T: Filter, F> {
    extract: T::Future,
    callback: F,
}

impl<T, F, E> Future for MapErrFuture<T, F>
where
    T: Filter,
    F: Fn(T::Error) -> E,
{
    type Item = T::Extract;
    type Error = E;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.extract.poll().map_err(|err| (self.callback)(err))
    }
}
