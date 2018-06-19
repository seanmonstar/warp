use ::filter::{Filter, FilterAnd};
use ::route::Route;

/// A filter that matches any route.
pub fn any() -> Any {
    Any {
        _inner: (),
    }
}

#[derive(Debug)]
pub struct Any {
    _inner: (),
}

impl Filter for Any {
    type Extract = ();

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        Some((route, ()))
    }
}

impl FilterAnd for Any {}

