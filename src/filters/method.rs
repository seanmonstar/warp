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

