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
/// - `warp::filters::reply::WithHeaders`
/// - `warp::filters::reply::WithDefaultHeader`
///
/// # Example
/// ```
/// use warp::{Filter, Wrap, Rejection};
/// use warp::filters::BoxedFilter;
///
/// struct SimpleLog;
/// impl <F> Wrap<F> for SimpleLog
/// where
///     F: Filter + Sized + Send + Sync + 'static,
///     F::Extract: Send,
///     F::Error: Into<Rejection>
/// {
///     type Wrapped = BoxedFilter<F::Extract>;
///
///     fn wrap(&self, filter: F) -> Self::Wrapped {
///         filter
///             .and(warp::any().map(|| log::info!("")))
///             .boxed()
///     }
/// }
///
/// let route = warp::any()
///    .map(warp::reply)
///    .with(SimpleLog);
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
