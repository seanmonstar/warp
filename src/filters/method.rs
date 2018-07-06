use http::Method;

use ::error::CombineError;
use ::filter::{And, Cons, Filter, FilterClone, filter_fn, filter_fn_cons, HList};
use ::never::Never;

/// Wrap a `Filter` in a new one that requires the request method to be `GET`.
pub fn get<F>(filter: F) -> And<impl FilterClone<Extract=(), Error=::Error>, F>
where
    F: Filter + Clone,
    F::Extract: HList,
    F::Error: CombineError<::Error>,
{
    method_is(Method::GET).and(filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `POST`.
pub fn post<F>(filter: F) -> And<impl FilterClone<Extract=(), Error=::Error>, F>
where
    F: Filter + Clone,
    F::Extract: HList,
    F::Error: CombineError<::Error>,
{
    method_is(Method::POST).and(filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `PUT`.
pub fn put<F>(filter: F) -> And<impl FilterClone<Extract=(), Error=::Error>, F>
where
    F: Filter + Clone,
    F::Extract: HList,
    F::Error: CombineError<::Error>,
{
    method_is(Method::PUT).and(filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `DELETE`.
pub fn delete<F>(filter: F) -> And<impl FilterClone<Extract=(), Error=::Error>, F>
where
    F: Filter + Clone,
    F::Extract: HList,
    F::Error: CombineError<::Error>,
{
    method_is(Method::DELETE).and(filter)
}

/// Extract the `Method` from the request.
pub fn method() -> impl Filter<Extract=Cons<Method>, Error=Never> + Copy {
    filter_fn_cons(|route| {
        Ok::<_, Never>(route.method().clone())
    })
}

fn method_is(method: Method) -> impl FilterClone<Extract=(), Error=::Error> {
    filter_fn(move |route| {
        trace!("method::{:?}?: {:?}", method, route.method());
        if route.method() == &method {
            Ok(())
        } else {
            //TODO: return method-specific error
            Err(::error::Kind::BadRequest.into())
        }
    })
}

