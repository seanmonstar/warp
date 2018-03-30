use std::ops::Add;

use ::Request;

pub mod method;
pub mod paths;

pub trait Filter {
    type Extract;
    fn filter(&self, input: &mut Request) -> FilterResult<Self::Extract>;

    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        And {
            first: self,
            second: other,
        }
    }
}

pub enum FilterResult<E> {
    Matched(E),
    Skipped,
}

pub struct And<T, U> {
    first: T,
    second: U,
}

impl<T, U> Filter for And<T, U>
where
    T: Filter,
    U: Filter,
{
    type Extract = (T::Extract, U::Extract);

    fn filter(&self, input: &mut Request) -> FilterResult<Self::Extract> {
        match self.first.filter(input) {
            FilterResult::Matched(extract1) => {
                match self.second.filter(input) {
                    FilterResult::Matched(extract2) => {
                        FilterResult::Matched((extract1, extract2))
                    },
                    FilterResult::Skipped => FilterResult::Skipped,
                }
            },
            FilterResult::Skipped => FilterResult::Skipped,
        }
    }
}

