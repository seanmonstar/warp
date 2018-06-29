use ::server::WarpService;

mod and;
mod map;
mod or;
mod service;
mod tuple;

use self::and::And;
use self::map::{Map, MapTuple};
pub(crate) use self::or::{Either, Or};
use self::service::FilteredService;
pub(crate) use self::tuple::{Combine, Cons, Func, HCons, HList, Tuple};

// A crate-private base trait, allowing the actual `filter` method to change
// signatures without it being a breaking change.
pub trait FilterBase {
    type Extract;

    fn filter(&self) -> Option<Self::Extract>;
}

impl<'a, T: FilterBase + 'a> FilterBase for &'a T {
    type Extract = T::Extract;

    fn filter(&self) -> Option<Self::Extract> {
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
/// let any = warp::any();
/// let closure = |route| {
///     any.filter(route)
/// };
/// ```
pub fn __warp_filter_compilefail_doctest() {}

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
    /// warp::path::exact("hello")
    ///     .and(warp::path::<String>());
    /// ```
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: FilterAnd + Sized,
        Self::Extract: HList + Combine<F::Extract>,
        F: Filter,
        F::Extract: HList,
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
    /// warp::path::<u32>()
    ///     .or(warp::path::<SocketAddr>());
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
    /// warp::path().map(|id: u64| {
    ///   format!("Hello #{}", id)
    /// });
    /// ```
    fn map<F>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        Self::Extract: HList,
        F: Func<<Self::Extract as HList>::Tuple>,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    /// Like `map`, but recevies a single tuple, and must return a single tuple.
    fn tmap<F, U>(self, fun: F) -> MapTuple<Self, F>
    where
        Self: Sized,
        Self::Extract: HList,
        F: Fn(<Self::Extract as HList>::Tuple) -> U,
        U: Tuple,
    {
        MapTuple {
            filter: self,
            callback: fun,
        }
    }

    #[doc(hidden)]
    fn service_with_not_found<N>(self, not_found: N) -> FilteredService<Self, N>
    where
        Self: Sized,
        //Self::Extract: Reply,
        N: WarpService,
    {
        FilteredService {
            filter: self,
            not_found,
        }
    }
}

impl<T: FilterBase> Filter for T {}

pub trait FilterAnd: Filter {}

fn _assert_object_safe() {
    fn _assert(_f: &Filter<Extract=()>) {}
}

pub fn filter_fn<F, U>(func: F) -> FilterFn<F>
where
    F: Fn() -> Option<U>,
    U: HList,
{
    FilterFn {
        func,
    }
}

pub fn filter_fn_cons<F, U>(func: F) -> FilterFn<impl Fn() -> Option<Cons<U>> + Copy>
where
    F: Fn() -> Option<U> + Copy,
{
    FilterFn {
        func: move || {
            func().map(|u| HCons(u, ()))
        },
    }
}

#[derive(Copy, Clone)]
pub struct FilterFn<F> {
    func: F,
}

impl<F, U> FilterBase for FilterFn<F>
where
    F: Fn() -> Option<U>,
    U: HList,
{
    type Extract = U;

    fn filter(&self) -> Option<Self::Extract> {
        (self.func)()
    }
}

impl<F, U> FilterAnd for FilterFn<F>
where
    F: Fn() -> Option<U>,
    U: HList,
{}
