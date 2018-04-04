use ::Request;
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

    fn filter(&self, input: &mut Request) -> Option<Self::Extract> {
        self.first
            .filter(input)
            .map(Either::A)
            .or_else(|| {
                self.second
                    .filter(input)
                    .map(Either::B)
            })
    }
}

