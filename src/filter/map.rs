use super::{Cons, FilterBase, Filter, FilterAnd, Func, HCons, HList, Tuple};

#[derive(Clone, Copy)]
pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for Map<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
{
    type Extract = Cons<F::Output>;
    fn filter(&self) -> Option<Self::Extract> {
        self.filter
            .filter()
            .map(|ex| HCons(self.callback.call(ex.flatten()), ()))
    }
}

impl<T, F> FilterAnd for Map<T, F>
where
    T: FilterAnd,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
{}


#[derive(Clone, Copy)]
pub struct MapTuple<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, U> FilterBase for MapTuple<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Fn(<T::Extract as HList>::Tuple) -> U,
    U: Tuple,
{
    type Extract = U::HList;
    fn filter(&self) -> Option<Self::Extract> {
        self.filter
            .filter()
            .map(|ex| (self.callback)(ex.flatten()).hlist())
    }
}

impl<T, F, U> FilterAnd for MapTuple<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Fn(<T::Extract as HList>::Tuple) -> U,
    U: Tuple,
{}
