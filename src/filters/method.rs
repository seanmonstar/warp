//! HTTP Method filters.
//!
//! The filters deal with the HTTP Method part of a request. Several here will
//! match the request `Method`, and if not matched, will reject the request
//! with a `405 Method Not Allowed`.
//!
//! There is also [`warp::method()`](method), which never rejects
//! a request, and just extracts the method to be used in your filter chains.
use http::Method;

use ::filter::{And, Filter, filter_fn, filter_fn_one, MapErr, One};
use ::never::Never;
use ::reject::{CombineRejection, Rejection};

/// Wrap a `Filter` in a new one that requires the request method to be `GET`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any().map(warp::reply);
/// let get_only = warp::get(route);
/// ```
pub fn get<F>(filter: F) -> And<
    impl Filter<Extract=(), Error=Rejection> + Copy,
    MapErr<F, fn(F::Error) -> <F::Error as CombineRejection<Rejection>>::Rejection>,
>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::GET)
        .and(filter.map_err(|err| err.cancel(::reject::method_not_allowed())))
}
/// Wrap a `Filter` in a new one that requires the request method to be `POST`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any().map(warp::reply);
/// let post_only = warp::post(route);
/// ```
pub fn post<F>(filter: F) -> And<
    impl Filter<Extract=(), Error=Rejection> + Copy,
    MapErr<F, fn(F::Error) -> <F::Error as CombineRejection<Rejection>>::Rejection>,
>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::POST)
        .and(filter.map_err(|err| err.cancel(::reject::method_not_allowed())))
}
/// Wrap a `Filter` in a new one that requires the request method to be `PUT`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any().map(warp::reply);
/// let put_only = warp::put(route);
/// ```
pub fn put<F>(filter: F) -> And<
    impl Filter<Extract=(), Error=Rejection> + Copy,
    MapErr<F, fn(F::Error) -> <F::Error as CombineRejection<Rejection>>::Rejection>,
>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::PUT)
        .and(filter.map_err(|err| err.cancel(::reject::method_not_allowed())))
}

/// Wrap a `Filter` in a new one that requires the request method to be `DELETE`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::any().map(warp::reply);
/// let delete_only = warp::delete(route);
/// ```
pub fn delete<F>(filter: F) -> And<
    impl Filter<Extract=(), Error=Rejection> + Copy,
    MapErr<F, fn(F::Error) -> <F::Error as CombineRejection<Rejection>>::Rejection>,
>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::DELETE)
        .and(filter.map_err(|err| err.cancel(::reject::method_not_allowed())))
}

/// Extract the `Method` from the request.
///
/// This never rejects a request.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::method()
///     .map(|method| {
///         format!("You sent a {} request!", method)
///     });
/// ```
pub fn method() -> impl Filter<Extract=One<Method>, Error=Never> + Copy {
    filter_fn_one(|route| {
        Ok::<_, Never>(route.method().clone())
    })
}

fn method_is<F>(func: F) -> impl Filter<Extract=(), Error=Rejection> + Copy
where
    F: Fn() -> &'static Method + Copy,
{
    filter_fn(move |route| {
        let method = func();
        trace!("method::{:?}?: {:?}", method, route.method());
        if route.method() == method {
            Ok(())
        } else {
            Err(::reject::method_not_allowed())
        }
    })
}

pub mod v2 {
    //! HTTP Method Filters
    //!
    //! These filters deal with the HTTP Method part of a request. They match
    //! the request `Method`, and if not matched, will reject the request with a
    //! `405 Method Not Allowed`.
    //!
    //! These "filters" behave a little differently than the rest. Instead of
    //! being used directly on requests, these filters "wrap" other filters.
    //!
    //!
    //! ## Wrapping a `Filter` (`with`)
    //!
    //! ```
    //! use warp::Filter;
    //! use warp::filters::method::v2 as methodv2;
    //!
    //! let route = warp::any()
    //!     .map(warp::reply);
    //!
    //! // This filter rejects any non-GET requests before they get to the
    //! // wrapped `route`.
    //! let get_only = route
    //!     .with(methodv2::get());
    //! ```
    //!
    //! Wrapping allows adding in conditional logic *before* the request enters
    //! the inner filter. In the example, it rejects non-GET requests before
    //! they get to the wrapped `route` filter.
    use futures::{future::FutureResult, IntoFuture};
    use http::Method;

    use filter::{And, Filter, FilterBase, MapErr, WrapSealed};
    use reject::{CombineRejection, Rejection};
    use reply::Reply;

    /// Wrap a `Filter` to require that the request method to be `GET`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    /// use warp::filters::method::v2 as methodv2;
    ///
    /// let route = warp::any().map(warp::reply);
    /// let get_only = route.with(methodv2::get());
    /// ```
    pub fn get() -> WithMethod {
        WithMethod(&Method::GET)
    }

    /// Wrap a `Filter` to require that the request method to be `POST`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    /// use warp::filters::method::v2 as methodv2;
    ///
    /// let route = warp::any().map(warp::reply);
    /// let post_only = route.with(methodv2::post());
    /// ```
    pub fn post() -> WithMethod {
        WithMethod(&Method::POST)
    }

    /// Wrap a `Filter` to require that the request method to be `PUT`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    /// use warp::filters::method::v2 as methodv2;
    ///
    /// let route = warp::any().map(warp::reply);
    /// let put_only = route.with(methodv2::put());
    /// ```
    pub fn put() -> WithMethod {
        WithMethod(&Method::PUT)
    }

    /// Wrap a `Filter` to require that the request method to be `DELETE`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    /// use warp::filters::method::v2 as methodv2;
    ///
    /// let route = warp::any().map(warp::reply);
    /// let delete_only = route.with(methodv2::delete());
    /// ```
    pub fn delete() -> WithMethod {
        WithMethod(&Method::DELETE)
    }

    /// Wrap a `Filter` to require a specific HTTP method.
    #[derive(Debug, Clone, Copy)]
    pub struct WithMethod(&'static Method);

    impl<F> WrapSealed<F> for WithMethod
    where
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: CombineRejection<Rejection>,
        <<F as FilterBase>::Error as CombineRejection<Rejection>>::Rejection:
            CombineRejection<Rejection>,
    {
        type Wrapped = And<
            MethodIs,
            MapErr<F, fn(F::Error) -> <F::Error as CombineRejection<Rejection>>::Rejection>,
        >;

        fn wrap(&self, filter: F) -> Self::Wrapped {
            MethodIs(self.0)
                .and(filter.map_err(|err| err.cancel(::reject::method_not_allowed())))
        }
    }

    /// A `Filter` that requires the request have a specific method.
    #[derive(Debug, Clone, Copy)]
    pub struct MethodIs(&'static Method);

    impl FilterBase for MethodIs where {
        type Extract = ();
        type Error = Rejection;
        type Future = FutureResult<(), Rejection>;

        fn filter(&self) -> Self::Future {
            let method = self.0;
            ::route::with(|route| {
                trace!("method::{:?}?: {:?}", method, route.method());
                if route.method() == method {
                    Ok(())
                } else {
                    Err(::reject::method_not_allowed())
                }
            }).into_future()
        }
    }
}
