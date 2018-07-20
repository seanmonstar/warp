mod and;
mod and_then;
mod map;
mod map_err;
mod or;
mod service;
mod wrap;

use futures::{future, Future, IntoFuture};

pub(crate) use ::generic::{Combine, One, one, Func, HList};
use ::reject::{CombineRejection, Reject};
use ::reply::Reply;
use ::route::{self, Route};
pub(crate) use self::and::And;
use self::and_then::AndThen;
pub(crate) use self::map::Map;
pub(crate) use self::map_err::MapErr;
pub(crate) use self::or::Or;
pub(crate) use self::wrap::{WrapSealed, Wrap};

// A crate-private base trait, allowing the actual `filter` method to change
// signatures without it being a breaking change.
pub trait FilterBase {
    type Extract;
    type Error: ::std::fmt::Debug + Send;
    type Future: Future<Item=Self::Extract, Error=Self::Error> + Send;

    fn filter(&self) -> Self::Future;

    // crate-private for now
    fn map_err<F, E>(self, fun: F) -> MapErr<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Error) -> E + Clone,
        E: ::std::fmt::Debug + Send,
    {
        MapErr {
            filter: self,
            callback: fun,
        }
    }
}

/* This may not actually make any sense...
impl<'a, T: FilterBase + 'a> FilterBase for &'a T {
    type Extract = T::Extract;
    type Error = T::Error;
    type Future = T::Future;

    fn filter(&self) -> Self::Future {
        (**self).filter()
    }
}
*/

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
    /// Additionally, this will join together the extracted values of both
    /// filters, so that `map` and `and_then` receive them as separate arguments.
    ///
    /// If a `Filter` extracts nothing (so, `()`), combining with any other
    /// filter will simply discard the `()`. If a `Filter` extracts one or
    /// more items, combining will mean it extracts the values of itself
    /// combined with the other.
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

    /// Composes this `Filter` with a function receiving the extracted value.
    ///
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
    ///
    /// # `Func`
    ///
    /// The generic `Func` trait is implemented for any function that receives
    /// the same arguments as this `Filter` extracts. In practice, this
    /// shouldn't ever bother you, and simply makes things feel more natural.
    ///
    /// For example, if three `Filter`s were combined together, suppose one
    /// extracts nothing (so `()`), and the other two extract two integers,
    /// a function that accepts exactly two integer arguments is allowed.
    /// Specifically, any `Fn(u32, u32)`.
    ///
    /// Without `Product` and `Func`, this would be a lot messier. First of
    /// all, the `()`s couldn't be discarded, and the tuples would be nested.
    /// So, instead, you'd need to pass an `Fn(((), (u32, u32)))`. That's just
    /// a single argument. Bleck!
    ///
    /// Even worse, the tuples would shuffle the types around depending on
    /// the exact invocation of `and`s. So, `unit.and(int).and(int)` would
    /// result in a different extracted type from `unit.and(int.and(int)`,
    /// or from `int.and(unit).and(int)`. If you changed around the order
    /// of filters, while still having them be semantically equivalent, you'd
    /// need to update all your `map`s as well.
    ///
    /// `Product`, `HList`, and `Func` do all the heavy work so that none of
    /// this is a bother to you. What's more, the types are enforced at
    /// compile-time, and tuple flattening is optimized away to nothing by
    /// LLVM.
    fn map<F>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Func<Self::Extract> + Clone,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }


    /// Composes this `Filter` with a function receiving the extracted value.
    ///
    /// The function should return some `IntoFuture` type.
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
        F: Func<Self::Extract> + Clone,
        F::Output: IntoFuture + Send,
        <F::Output as IntoFuture>::Error: CombineRejection<Self::Error>,
        <F::Output as IntoFuture>::Future: Send,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }

    /// Wraps the current filter with some wrapper.
    ///
    /// The wrapper may do some preparation work before starting this filter,
    /// and may do post-processing after the filter completes.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let route = warp::any()
    ///     .map(warp::reply);
    ///
    /// // Wrap the route with a log wrapper.
    /// let route = route.with(warp::log("example"));
    /// ```
    fn with<W>(self, wrapper: W) -> W::Wrapped
    where
        Self: Sized,
        W: Wrap<Self>,
    {
        wrapper.wrap(self)
    }
}

impl<T: FilterBase> Filter for T {}

pub trait FilterClone: Filter + Clone {}

impl<T: Filter + Clone> FilterClone for T {}

// ===== FilterReply =====

// This is a hack to mimic a trait alias, such that if a user has some
// `impl Filter` that extracts some `impl Reply`, they can still use the
// type system to return the type. Without this, it's currently illegal
// to return an `impl Filter<Extract = impl Reply>`.
//
// So, instead, they can write `impl FilterReply`.
//
// The associated types here technically leak, (so, you could technically
// type `impl FilterReply<__DontNameMeReply = StatusCode>`, but hopefully
// the name will tell people to expect to be broken if they do.
pub trait FilterReplyBase: Filter {
    type __DontNameMeReply: Reply + Send;
    type __DontNameMeReject: Reject + Send;
    type __DontNameMeFut: Future<Item = Self::__DontNameMeReply, Error = Self::__DontNameMeReject> + Send + 'static;

    fn reply(&self) -> Self::__DontNameMeFut;
}

impl<T> FilterReplyBase for T
where
    T: Filter,
    T::Extract: Reply + Send,
    T::Error: Reject + Send,
    T::Future: 'static,
{
    type __DontNameMeReply = T::Extract;
    type __DontNameMeReject = T::Error;
    type __DontNameMeFut = T::Future;

    fn reply(&self) -> Self::__DontNameMeFut {
        self.filter()
    }
}

/// A form of "trait alias" of `Filter`.
///
/// Specifically, for any type that implements `Filter`, and extracts from
/// type that implements `Reply`, and an error that implements `Reject`,
/// automatically implements `FilterReply`.
///
/// The usefulness of this alias is to allow applications to construct filters
/// in some function, and return them, before starting a server that uses them.
/// These types can then be used in unit tests without having to start the
/// server.
pub trait FilterReply: FilterReplyBase {}

impl<T: FilterReplyBase> FilterReply for T {}


fn _assert_object_safe() {
    fn _assert(_f: &Filter<Extract=(), Error=(), Future=future::FutureResult<(), ()>>) {}
}

// ===== FilterFn =====

pub(crate) fn filter_fn<F, U>(func: F) -> FilterFn<F>
where
    F: Fn(&mut Route) -> U,
    U: IntoFuture,
    U::Item: HList,
{
    FilterFn {
        func,
    }
}

pub(crate) fn filter_fn_one<F, U>(func: F)
    -> FilterFn<impl Fn(&mut Route) -> future::Map<U::Future, fn(U::Item) -> One<U::Item>> + Copy>
where
    F: Fn(&mut Route) -> U + Copy,
    U: IntoFuture,
{
    filter_fn(move |route| {
        func(route)
            .into_future()
            .map(one as _)
    })
}

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub(crate) struct FilterFn<F> {
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

