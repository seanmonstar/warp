mod and;
mod map;
mod or;
mod service;
mod tuple;

use futures::{future, Async, Future, IntoFuture, Poll};

use ::reject::CombineRejection;
use ::route::Route;
pub(crate) use self::and::And;
use self::map::Map;
pub(crate) use self::or::{Either, Or};
pub(crate) use self::tuple::{Combine, Cons, cons, Func, HCons, HList};

// A crate-private base trait, allowing the actual `filter` method to change
// signatures without it being a breaking change.
pub trait FilterBase {
    type Extract;
    type Error: ::std::fmt::Debug + Send;
    type Future: Future<Item=Extracted<Self::Extract>, Error=Errored<Self::Error>> + Send;

    fn filter(&self, route: Route) -> Self::Future;
}

impl<'a, T: FilterBase + 'a> FilterBase for &'a T {
    type Extract = T::Extract;
    type Error = T::Error;
    type Future = T::Future;

    fn filter(&self, route: Route) -> Self::Future {
        (**self).filter(route)
    }
}

pub struct Extracted<T>(Route, T);

pub struct Errored<E>(Route, E);

impl<T> Extracted<T> {
    #[inline]
    pub(crate) fn item(self) -> T {
        self.1
    }

    #[inline]
    pub(crate) fn map<F, U>(self, func: F) -> Extracted<U>
    where
        F: FnOnce(T) -> U,
    {
        let u = func(self.1);
        Extracted(self.0, u)
    }
}

impl<E> Errored<E> {
    #[inline]
    pub(crate) fn error(self) -> E {
        self.1
    }

    pub(crate) fn combined<U>(self) -> Errored<U::Rejection>
    where
        U: CombineRejection<E>,
    {
        Errored(self.0, self.1.into())
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
/// let _ = |route| {
///     warp::any().filter(route)
/// };
/// ```
pub fn __warp_filter_compilefail_doctest() {
    // Duplicate code to make sure the code is otherwise valid.
    let _ = |route| {
        ::any().filter(route)
    };
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

    /*
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
    */
}

impl<T: FilterBase> Filter for T {}

pub trait FilterClone: Filter + Clone {}

impl<T: Filter + Clone> FilterClone for T {}

fn _assert_object_safe() {
    fn _assert(_f: &Filter<Extract=(), Error=(), Future=future::FutureResult<(), ()>>) {}
}

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
pub struct FilterFn<F> {
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
    type Future = FilterFnFuture<U::Future>;

    #[inline]
    fn filter(&self, mut route: Route) -> Self::Future {
        let inner = (self.func)(&mut route).into_future();
        FilterFnFuture {
            inner,
            route: Some(route),
        }
    }
}

pub struct FilterFnFuture<F> {
    inner: F,
    route: Option<Route>,
}

impl<F> Future for FilterFnFuture<F>
where
    F: Future + Send,
{
    type Item = Extracted<F::Item>;
    type Error = Errored<F::Error>;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(item)) => {
                Ok(Async::Ready(Extracted(self.route.take().expect("polled after complete"), item)))
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(err) => {
                Err(Errored(self.route.take().expect("polled after complete"), err))
            }
        }
    }
}

