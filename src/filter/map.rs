use ::Request;
use super::Filter;

pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, U> Filter for Map<T, F>
where
    T: Filter,
    F: Fn(T::Extract) -> U,
{
    type Extract = U;
    fn filter(&self, input: &mut Request) -> Option<U> {
        self.filter
            .filter(input)
            .map(|ex| (self.callback)(ex))
    }
}
