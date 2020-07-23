//! Variable Filter

use crate::filter::Filter;
use std::convert::Infallible;

/// Creates a `Filter` that clones in its given variable into each request
pub fn with_var<T>(var: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone
where
    T: Clone + Send,
{
    // The clone is needed since the produced closure will be run multiple times
    super::any::any().map(move || var.clone())
}
