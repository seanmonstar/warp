use ::reply::Reply;
use ::server::WarpService;

mod and;
mod map;
mod or;
mod service;

use self::and::{And, AndUnit, UnitAnd};
use self::map::Map;
pub(crate) use self::or::{Either, Or};
use self::service::FilteredService;

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

pub trait Filter: FilterBase {
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: FilterAnd + Sized,
        F: Filter,
    {
        And {
            first: self,
            second: other,
        }
    }

    fn unit_and<F>(self, other: F) -> UnitAnd<Self, F>
    where
        Self: Filter<Extract=()> + FilterAnd + Sized,
        F: Filter,
    {
        UnitAnd {
            first: self,
            second: other,
        }
    }

    fn and_unit<F>(self, other: F) -> AndUnit<Self, F>
    where
        Self: FilterAnd + Sized,
        F: Filter<Extract=()>,
    {
        AndUnit {
            first: self,
            second: other,
        }
    }

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

    fn map<F, U>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Extract) -> U,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    fn service_with_not_found<N>(self, not_found: N) -> FilteredService<Self, N>
    where
        Self: Sized,
        Self::Extract: Reply,
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
