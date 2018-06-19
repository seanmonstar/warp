use ::route;
use super::{FilterBase, Filter, FilterAnd};

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

impl<T, U> FilterBase for Or<T, U>
where
    T: Filter,
    U: Filter,
{
    type Extract = Either<T::Extract, U::Extract>;

    fn filter(&self) -> Option<Self::Extract> {
        route::with(|route| {
            let txn = route.transaction();
            if let Some(ex) = self.first.filter() {
                // txn implicitly commited
                return Some(Either::A(ex))
            }

            // revert any changes made to route
            txn.revert(route);

            if let Some(ex) = self.second.filter() {
                // txn implicitly commited
                return Some(Either::B(ex))
            }

            txn.revert(route);
            None
        })
    }
}

impl<T: FilterAnd, U: FilterAnd> FilterAnd for Or<T, U> {}

