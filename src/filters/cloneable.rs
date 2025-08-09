//! A filter that matches any route and supplies a clone of the given
//! [Cloneable](Clone)
use crate::Filter;
use std::convert::Infallible;

/// A [`Filter`](Filter) that matches any route and yields a clone
/// of the given [Cloneable](Clone).
///
/// This can be used to supply services or tools to the handling methods.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use warp::Filter;
///
/// let state = Arc::new(vec![33, 41]);
/// let with_state = warp::with_cloneable(state);
///
/// let route = warp::path::param()
///     .and(with_state)
///     .map(|param_id: u32, db: Arc<Vec<u32>>| {
///         db.contains(&param_id)
///     });
/// ```
pub fn with_cloneable<C: Clone + Send>(
    value: C,
) -> impl Filter<Extract = (C,), Error = Infallible> + Clone {
    crate::any().map(move || value.clone())
}
