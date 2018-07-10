mod and;
mod and_then;
mod map;
mod or;
mod service;
mod tuple;

use futures::{future, Future, IntoFuture};

use ::reject::CombineRejection;
use ::route::{self, Route};
pub(crate) use self::and::And;
use self::and_then::AndThen;
use self::map::Map;
pub(crate) use self::or::{Either, Or};
pub(crate) use self::tuple::{Combine, Cons, cons, Func, HCons, HList};

// A crate-private base trait, allowing the actual `filter` method to change
// signatures without it being a breaking change.
pub trait FilterBase {
    type Extract;
    type Error: ::std::fmt::Debug + Send;
    type Future: Future<Item=Self::Extract, Error=Self::Error> + Send;

    fn filter(&self) -> Self::Future;
}

impl<'a, T: FilterBase + 'a> FilterBase for &'a T {
    type Extract = T::Extract;
    type Error = T::Error;
    type Future = T::Future;

    fn filter(&self) -> Self::Future {
        (**self).filter()
    }
}

/// This just makes use of rustdoc's ability to make compile_fail tests.
/// This is specifically testing to make sure `Filter::filter` isn't
/// able to be called from outside the crate (since rustdoc tests are
/// compiled as new crates).
///
/// ```compile_fail
/// use warp::Filter;
///
/// let _ = warp::any().filter();
/// ```
pub fn __warp_filter_compilefail_doctest() {
    // Duplicate code to make sure the code is otherwise valid.
    let _ = ::any().filter();
}

/// Composable request filters.
pub trait Filter: FilterBase {
    /// Composes a new `Filter` that requires both this and the other to filter a request.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// // Match `/hello/:name`...
    /// warp::path("hello")
    ///     .and(warp::path::param::<String>());
    /// ```
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        Self::Extract: HList + Combine<F::Extract>,
        F: Filter + Clone,
        F::Extract: HList,
        F::Error: CombineRejection<Self::Error>,
    {
        And {
            first: self,
            second: other,
        }
    }

    /// Composes a new `Filter` of either this or the other filter.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::SocketAddr;
    /// use warp::Filter;
    ///
    /// // Match either `/:u32` or `/:socketaddr`
    /// warp::path::param::<u32>()
    ///     .or(warp::path::param::<SocketAddr>());
    /// ```
    fn or<F>(self, other: F) -> Or<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        Or {
            first: self,
            second: other,
        }
    }

    /// Composes this `Filter` with a closure receiving the extracted value from this.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// // Map `/:id`
    /// warp::path::param().map(|id: u64| {
    ///   format!("Hello #{}", id)
    /// });
    /// ```
    fn map<F>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        Self::Extract: HList,
        F: Func<<Self::Extract as HList>::Tuple> + Clone,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    /// Composes this `Filter` with a closure receiving the extracted value from this.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// // Validate after `/:id`
    /// warp::path::param().and_then(|id: u64| {
    ///     if id != 0 {
    ///         Ok(format!("Hello #{}", id))
    ///     } else {
    ///         Err(warp::reject())
    ///     }
    /// });
    /// ```
    fn and_then<F>(self, fun: F) -> AndThen<Self, F>
    where
        Self: Sized,
        Self::Extract: HList,
        F: Func<<Self::Extract as HList>::Tuple> + Clone,
        F::Output: IntoFuture + Send,
        <F::Output as IntoFuture>::Error: CombineRejection<Self::Error>,
        <F::Output as IntoFuture>::Future: Send,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }
}

impl<T: FilterBase> Filter for T {}

pub trait FilterClone: Filter + Clone {}

impl<T: Filter + Clone> FilterClone for T {}

fn _assert_object_safe() {
    fn _assert(_f: &Filter<Extract=(), Error=(), Future=future::FutureResult<(), ()>>) {}
}

// ===== FilterFn =====

pub fn filter_fn<F, U>(func: F) -> FilterFn<F>
where
    F: Fn(&mut Route) -> U,
    U: IntoFuture,
    U::Item: HList,
{
    FilterFn {
        func,
    }
}

pub fn filter_fn_cons<F, U>(func: F)
    -> FilterFn<impl Fn(&mut Route) -> future::Map<U::Future, fn(U::Item) -> Cons<U::Item>> + Copy>
where
    F: Fn(&mut Route) -> U + Copy,
    U: IntoFuture,
{
    filter_fn(move |route| {
        func(route)
            .into_future()
            .map(cons as _)
    })
}

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct FilterFn<F> {
    // TODO: could include a `debug_str: &'static str` to be used in Debug impl
    func: F,
}

impl<F, U> FilterBase for FilterFn<F>
where
    F: Fn(&mut Route) -> U,
    U: IntoFuture,
    U::Future: Send,
    U::Item: HList,
    U::Error: ::std::fmt::Debug + Send,
{
    type Extract = U::Item;
    type Error = U::Error;
    type Future = U::Future;

    #[inline]
    fn filter(&self) -> Self::Future {
        route::with(|route| {
            (self.func)(route).into_future()
        })
    }
}

