use futures::{Async, Future, Poll};

use super::{Cons, FilterBase, Filter, Func, HCons, HList};

#[derive(Clone, Copy)]
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
    type Future = MapFuture<T::Future, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        MapFuture {
            extract: self.filter.filter(),
            callback: self.callback.clone(),
        }
    }
}

pub struct MapFuture<T, F> {
    extract: T,
    callback: F,
}

impl<T, F> Future for MapFuture<T, F>
where
    T: Future,
    T::Item: HList,
    F: Func<<T::Item as HList>::Tuple>,
{
    type Item = Cons<F::Output>;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let item = try_ready!(self.extract.poll());
        Ok(Async::Ready(HCons(self.callback.call(item.flatten()), ())))
    }
}

#[derive(Clone, Copy)]
pub struct MapTuple<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

/*
impl<T, F, U> FilterBase for MapTuple<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Fn(<T::Extract as HList>::Tuple) -> U,
    U: Tuple,
{
    type Extract = U::HList;
    #[inline]
    fn filter(&self) -> Self::Extract {
        self.filter
            .filter()
            .map(|ex| (self.callback)(ex.flatten()).hlist())
    }
}
*/

