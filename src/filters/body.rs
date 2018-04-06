//! Body filters
//!
//! Filters that extract a body for a route.
//!
//! # Body filters must "end" a filter chain
//!
//! ```compile_fail
//! let a = warp::body::concat();
//! let b = warp::body::concat();
//!
//! // Cannot chain something after 'a'
//! a.and(b)
//! ```
use ::filter::Filter;
use ::route::Route;

pub fn concat() -> Concat {
    Concat {
        _i: (),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Concat {
    _i: (),
}

pub struct ConcatFut;

impl Filter for Concat {
    type Extract = ConcatFut;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, Self::Extract)> {
        route.take_body()
            .map(|(route, _body)| (route, ConcatFut))
    }
}
