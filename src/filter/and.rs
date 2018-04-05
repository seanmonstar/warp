use ::route::Route;
use super::Filter;

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

impl<T, U> Filter for And<T, U>
where
    T: Filter,
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

impl<T, U> Filter for UnitAnd<T, U>
where
    T: Filter<Extract=()>,
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

impl<T, U> Filter for AndUnit<T, U>
where
    T: Filter,
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

