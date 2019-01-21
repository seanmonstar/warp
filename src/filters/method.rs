//! HTTP Method filters.
//!
//! The filters deal with the HTTP Method part of a request. Several here will
//! match the request `Method`, and if not matched, will reject the request
//! with a `405 Method Not Allowed`.
//!
//! There is also [`warp::method()`](method), which never rejects
//! a request, and just extracts the method to be used in your filter chains.
use http::Method;

use filter::{filter_fn, filter_fn_one, And, Filter, One};
use never::Never;
use reject::{CombineRejection, Rejection};

pub use self::v2::{
    delete as delete2, get as get2, head, options, patch, post as post2, put as put2,
};

#[doc(hidden)]
#[deprecated(note = "warp::get2() is meant to replace get()")]
pub fn get<F>(filter: F) -> And<impl Filter<Extract = (), Error = Rejection> + Copy, F>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::GET).and(filter)
}

#[doc(hidden)]
#[deprecated(note = "warp::post2() is meant to replace post()")]
pub fn post<F>(filter: F) -> And<impl Filter<Extract = (), Error = Rejection> + Copy, F>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::POST).and(filter)
}

#[doc(hidden)]
#[deprecated(note = "warp::put2() is meant to replace put()")]
pub fn put<F>(filter: F) -> And<impl Filter<Extract = (), Error = Rejection> + Copy, F>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::PUT).and(filter)
}

#[doc(hidden)]
#[deprecated(note = "warp::delete2() is meant to replace delete()")]
pub fn delete<F>(filter: F) -> And<impl Filter<Extract = (), Error = Rejection> + Copy, F>
where
    F: Filter + Clone,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    method_is(|| &Method::DELETE).and(filter)
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
pub fn method() -> impl Filter<Extract = One<Method>, Error = Never> + Copy {
    filter_fn_one(|route| Ok::<_, Never>(route.method().clone()))
}

// NOTE: This takes a static function instead of `&'static Method` directly
// so that the `impl Filter` can be zero-sized. Moving it around should be
// cheaper than holding a single static pointer (which would make it 1 word).
fn method_is<F>(func: F) -> impl Filter<Extract = (), Error = Rejection> + Copy
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
    use http::Method;

    use filter::Filter;
    use reject::Rejection;

    use super::method_is;

    /// Create a `Filter` that requires the request method to be `GET`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let get_only = warp::get2().map(warp::reply);
    /// ```
    pub fn get() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::GET)
    }

    /// Create a `Filter` that requires the request method to be `POST`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let post_only = warp::post2().map(warp::reply);
    /// ```
    pub fn post() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::POST)
    }

    /// Create a `Filter` that requires the request method to be `PUT`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let put_only = warp::put2().map(warp::reply);
    /// ```
    pub fn put() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::PUT)
    }

    /// Create a `Filter` that requires the request method to be `DELETE`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let delete_only = warp::delete2().map(warp::reply);
    /// ```
    pub fn delete() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::DELETE)
    }

    /// Create a `Filter` that requires the request method to be `HEAD`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let head_only = warp::head().map(warp::reply);
    /// ```
    pub fn head() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::HEAD)
    }

    /// Create a `Filter` that requires the request method to be `OPTIONS`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let options_only = warp::options().map(warp::reply);
    /// ```
    pub fn options() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::OPTIONS)
    }

    /// Create a `Filter` that requires the request method to be `PATCH`.
    ///
    /// # Example
    ///
    /// ```
    /// use warp::Filter;
    ///
    /// let patch_only = warp::patch().map(warp::reply);
    /// ```
    pub fn patch() -> impl Filter<Extract = (), Error = Rejection> + Copy {
        method_is(|| &Method::PATCH)
    }
}
