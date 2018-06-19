use ::route::Route;
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
    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, U)> {
        self.filter
            .filter(route)
            .map(|(route, ex)| (route, (self.callback)(ex)))
    }
}

impl<T: FilterAnd, F: Fn(T::Extract) -> U, U> FilterAnd for Map<T, F> {}
