use ::route::Route;
use super::{Filter, FilterAnd};

#[derive(Clone, Copy, Debug)]
pub struct Or<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

#[derive(Debug)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

impl<T, U> Filter for Or<T, U>
where
    T: Filter,
    U: Filter,
{
    type Extract = Either<T::Extract, U::Extract>;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        let (route, opt) = route.scoped(|route| {
            self.first
                .filter(route)
        });
        if let Some(ex) = opt {
            Some((route, Either::A(ex)))
        } else {
            self.second
                .filter(route)
                .map(|(route, ex)| (route, Either::B(ex)))
        }
    }
}

impl<T: FilterAnd, U: FilterAnd> FilterAnd for Or<T, U> {}

