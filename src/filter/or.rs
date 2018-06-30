use ::route;
use super::{Cons, FilterBase, Filter, HCons};

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
    type Extract = Cons<
        Either<
            T::Extract,
            U::Extract,
        >
    >;

    fn filter(&self) -> Option<Self::Extract> {
        route::with(|route| {
            route
                .transaction(|| {
                    self.first.filter()
                })
                .map(Either::A)
                .or_else(|| {
                    route.transaction(|| {
                        self
                            .second
                            .filter()
                            .map(Either::B)
                    })
                })
                .map(|e| HCons(e, ()))
        })
    }
}

