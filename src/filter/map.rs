use super::{FilterBase, Filter, FilterAnd};

pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, U> FilterBase for Map<T, F>
where
    T: Filter,
    F: Fn(T::Extract) -> U,
{
    type Extract = U;
    fn filter(&self) -> Option<U> {
        self.filter
            .filter()
            .map(|ex| (self.callback)(ex))
    }
}

impl<T: FilterAnd, F: Fn(T::Extract) -> U, U> FilterAnd for Map<T, F> {}
