use http::Method;

use ::error::CombineError;
use ::filter::{Cons, FilterBase, Filter, filter_fn_cons};
use ::never::Never;
use ::route::Route;

/// Wrap a `Filter` in a new one that requires the request method to be `GET`.
pub fn get<F>(filter: F) -> MethodIs<F>
where
    F: Filter,
    F::Error: CombineError<::never::Never>,
{
    method_is(Method::GET, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `POST`.
pub fn post<F>(filter: F) -> MethodIs<F>
where
    F: Filter,
    F::Error: CombineError<::never::Never>,
{
    method_is(Method::POST, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `PUT`.
pub fn put<F>(filter: F) -> MethodIs<F>
where
    F: Filter,
    F::Error: CombineError<::never::Never>,
{
    method_is(Method::PUT, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `DELETE`.
pub fn delete<F>(filter: F) -> MethodIs<F>
where
    F: Filter,
    F::Error: CombineError<::never::Never>,
{
    method_is(Method::DELETE, filter)
}

/// Extract the `Method` from the request.
pub fn method() -> impl Filter<Extract=Cons<Method>> + Copy {
    filter_fn_cons(|route| {
        Ok::<_, Never>(route.method().clone())
    })
}

#[derive(Clone)]
pub struct MethodIs<F> {
    method: Method,
    filter: F,
}

impl<F: Filter> FilterBase for MethodIs<F> {
    type Extract = F::Extract;
    type Error = F::Error;
    type Future = F::Future;

    fn filter(&self, route: Route) -> Self::Future {
        trace!("method::{:?}?: {:?}", self.method, route.method());
        if &self.method == route.method() {
            self.filter.filter(route)
        } else {
            unimplemented!()
            //TODO: return method-specific error
            //Err(Error(()))
        }
    }
}

fn method_is<F>(method: Method, filter: F) -> MethodIs<F>
where
    F: Filter,
    F::Error: CombineError<::never::Never>,
{
    MethodIs {
        method,
        filter,
    }
}

