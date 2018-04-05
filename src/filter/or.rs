use ::route::Route;
use super::Filter;

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
        self.first
            .filter(route.clone())
            .map(|(route, ex)| (route, Either::A(ex)))
            .or_else(|| {
                self.second
                    .filter(route)
                    .map(|(route, ex)| (route, Either::B(ex)))
            })
    }
}

