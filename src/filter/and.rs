use super::{Combine, FilterBase, Filter, FilterAnd};

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> FilterBase for And<T, U>
where
    T: FilterAnd,
    T::Extract: Combine<U::Extract>,
    U: Filter,
{
    type Extract = <T::Extract as Combine<U::Extract>>::Output;

    fn filter(&self) -> Option<Self::Extract> {
        self.first
            .filter()
            .and_then(|ex1| {
                self.second
                    .filter()
                    .map(|ex2| ex1.combine(ex2))
            })
    }
}

impl<T, U> FilterAnd for And<T, U>
where
    T: FilterAnd,
    T::Extract: Combine<U::Extract>,
    U: Filter,
{}

