use ::route::Route;
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

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        self.first
            .filter(route)
            .and_then(|(route, ex1)| {
                self.second
                    .filter(route)
                    .map(|(route, ex2)| (route, (ex1, ex2)))
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

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        self.first
            .filter(route)
            .and_then(|(route, ())| {
                self.second
                    .filter(route)
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

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        self.first
            .filter(route)
            .and_then(|(route, ex)| {
                self.second
                    .filter(route)
                    .map(|(route, ())| (route, ex))
            })
    }
}

impl<T: FilterAnd, U: FilterAnd<Extract=()>> FilterAnd for AndUnit<T, U> {}

