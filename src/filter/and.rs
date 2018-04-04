use ::Request;
use super::Filter;

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> Filter for And<T, U>
where
    T: Filter,
    U: Filter,
{
    type Extract = (T::Extract, U::Extract);

    fn filter(&self, input: &mut Request) -> Option<Self::Extract> {
        self.first
            .filter(input)
            .and_then(|ex1| {
                self.second
                    .filter(input)
                    .map(|ex2| (ex1, ex2))
            })
    }
}

