use super::Filter;

pub trait WrapSealed<F: Filter> {
    type Wrapped: Filter;

    fn wrap(&self, filter: F) -> Self::Wrapped;
}

impl<'a, T, F> WrapSealed<F> for &'a T
where
    T: WrapSealed<F>,
    F: Filter,
{
    type Wrapped = T::Wrapped;
    fn wrap(&self, filter: F) -> Self::Wrapped {
        (*self).wrap(filter)
    }
}

pub trait Wrap<F: Filter>: WrapSealed<F> {}

impl<T, F> Wrap<F> for T
where
    T: WrapSealed<F>,
    F: Filter,
{
}
