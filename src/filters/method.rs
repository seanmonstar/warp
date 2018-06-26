use http;

use ::filter::{FilterBase, Filter, FilterAnd};
use ::route;

/// Wrap a `Filter` in a new one that requires the request method to be `GET`.
pub fn get<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::GET, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `POST`.
pub fn post<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::POST, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `PUT`.
pub fn put<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::PUT, filter)
}

/// Wrap a `Filter` in a new one that requires the request method to be `DELETE`.
pub fn delete<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::DELETE, filter)
}

pub struct Method<F> {
    m: http::Method,
    next: F,
}

impl<F: Filter> Method<F> {
    pub fn new(method: http::Method, filter: F) -> Self {
        Self {
            m: method,
            next: filter,
        }
    }
}

impl<F: Filter> FilterBase for Method<F> {
    type Extract = F::Extract;

    fn filter(&self) -> Option<F::Extract> {
        route::with(|route| {
            trace!("method::{:?}: {:?}", self.m, route.method());
            if &self.m == route.method() {
                self.next.filter()
            } else {
                None
            }
        })
    }
}

impl<F: FilterAnd> FilterAnd for Method<F> {}

