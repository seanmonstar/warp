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

/// Function that receives a filter to be combined with pre and after filters
pub fn wrap_fn<F, T, U>(func: F) -> WrapFn<F>
where
    F: Fn(T) -> U,
    T: Filter,
    U: Filter,
{
    WrapFn { func }
}

#[derive(Debug)]
pub struct WrapFn<F> {
    func: F,
}

impl<F, T, U> WrapSealed<T> for WrapFn<F>
where
    F: Fn(T) -> U,
    T: Filter,
    U: Filter,
{
    type Wrapped = U;

    fn wrap(&self, filter: T) -> Self::Wrapped {
        (self.func)(filter)
    }
}
