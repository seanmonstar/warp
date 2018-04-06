use http;

use ::filter::{Filter, FilterAnd};
use ::route::Route;

pub fn get<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::GET, filter)
}

pub fn post<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::POST, filter)
}

pub fn put<F: Filter>(filter: F) -> Method<F> {
    Method::new(http::Method::PUT, filter)
}

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

impl<F: Filter> Filter for Method<F> {
    type Extract = F::Extract;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, F::Extract)> {
        trace!("method::{:?}: {:?}", self.m, route.method());
        if &self.m == route.method() {
            self.next.filter(route)
        } else {
            None
        }
    }
}

impl<F: FilterAnd> FilterAnd for Method<F> {}
