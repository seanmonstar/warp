use super::{FilterBase, Filter, FilterAnd};

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

#[derive(Clone, Copy, Debug)]
pub struct UnitAnd<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

#[derive(Clone, Copy, Debug)]
pub struct AndUnit<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> FilterBase for And<T, U>
where
    T: FilterAnd,
    U: Filter,
{
    type Extract = (T::Extract, U::Extract);

    fn filter(&self) -> Option<Self::Extract> {
        self.first
            .filter()
            .and_then(|ex1| {
                self.second
                    .filter()
                    .map(|ex2| (ex1, ex2))
            })
    }
}

impl<T: FilterAnd, U: FilterAnd> FilterAnd for And<T, U> {}

impl<T, U> FilterBase for UnitAnd<T, U>
where
    T: FilterAnd<Extract=()>,
    U: Filter,
{
    type Extract = U::Extract;

    fn filter<'a>(&self) -> Option<Self::Extract> {
        self.first
            .filter()
            .and_then(|()| {
                self.second
                    .filter()
            })
    }
}

impl<T: FilterAnd<Extract=()>, U: FilterAnd> FilterAnd for UnitAnd<T, U> {}

impl<T, U> FilterBase for AndUnit<T, U>
where
    T: FilterAnd,
    U: Filter<Extract=()>,
{
    type Extract = T::Extract;

    fn filter(&self) -> Option<Self::Extract> {
        self.first
            .filter()
            .and_then(|ex| {
                self.second
                    .filter()
                    .map(move |()| ex)
            })
    }
}

impl<T: FilterAnd, U: FilterAnd<Extract=()>> FilterAnd for AndUnit<T, U> {}

