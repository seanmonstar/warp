use super::Filter;

/// Types that allow customizing Filters by wrapping them.
///
/// Types that implement `Wrap` can be used with the `.with()` filter combinator.
///
/// Notable uses of this trait include:
///
/// - `warp::filters::log::Log`
/// - `warp::filters::compression::Compression`
/// - `warp::filters::cors::Cors`
/// - `warp::filters::cors::Builder`
/// - `warp::filters::reply::WithHeader`
/// - `warp::filters::reply::WithHeaderS`
/// - `warp::filters::reply::WithDefaultHeader`
///
/// # Example
/// ```
/// // use warp::{Filter, Wrap};
///
/// // let route = warp::any()
/// //    .map(warp::reply)
/// //    .with(unimplemented!());
/// ```
pub trait Wrap<F: Filter> {
    /// The type of the Filter produced by wrapping another Filter.
    type Wrapped: Filter;

    /// Wraps the Filter with the associated type that implements Filter.
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
