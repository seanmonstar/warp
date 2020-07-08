use super::Filter;

/// Wraps filters.
pub trait Wrap<F: Filter> {
    /// The type of the Filter produced by wrapping another Filter.
    type Wrapped: Filter;

    /// Wraps the filter.
    fn wrap(&self, filter: F) -> Self::Wrapped;
}

impl<'a, T, F> Wrap<F> for &'a T
where
    T: Wrap<F>,
    F: Filter,
{
    type Wrapped = T::Wrapped;
    fn wrap(&self, filter: F) -> Self::Wrapped {
        (*self).wrap(filter)
    }
}
