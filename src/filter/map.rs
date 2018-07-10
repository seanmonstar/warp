use futures::{Async, Future, Poll};

use super::{Cons, FilterBase, Filter, Func, cons, HList};

#[derive(Clone, Copy, Debug)]
pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for Map<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple> + Clone + Send,
{
    type Extract = Cons<F::Output>;
    type Error = T::Error;
    type Future = MapFuture<T, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        MapFuture {
            extract: self.filter.filter(),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct MapFuture<T: Filter, F> {
    extract: T::Future,
    callback: F,
}

impl<T, F> Future for MapFuture<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
{
    type Item = Cons<F::Output>;
    type Error = T::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let ex = try_ready!(self.extract.poll());
        let ex = cons(self.callback.call(ex.flatten()));
        Ok(Async::Ready(ex))
    }
}

