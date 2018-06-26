//! dox?
use std::marker::PhantomData;
use std::str::FromStr;

use ::filter::{FilterBase, FilterAnd};
use ::route;

/// Return an extract `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name,
/// and try to parse to a `T`, otherwise rejects the request.
pub fn header<T>(name: &'static str) -> Extract<T> {
    Extract {
        name,
        _marker: PhantomData,
    }
}

/// Return an exact `Filter` for a specific header name.
///
/// This `Filter` will look for a header with supplied name and
/// the exact value, otherwise rejects the request.
pub fn exact(name: &'static str, value: &'static str) -> Exact {
    Exact {
        name,
        value,
    }
}

/// dox?
#[derive(Clone, Debug)]
pub struct Exact {
    name: &'static str,
    value: &'static str,
}

impl FilterBase for Exact {
    type Extract = ();

    fn filter(&self) -> Option<()> {
        trace!("header::Exact({:?}, {:?})", self.name, self.value);
        route::with(|route| {
            route.headers()
                .get(self.name)
                .and_then(|val| {
                    if val == self.value {
                        Some(())
                    } else {
                        None
                    }
                })
        })
    }
}

impl FilterAnd for Exact {}

/// dox?
pub struct Extract<T> {
    name: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> FilterBase for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter(&self) -> Option<T> {
        trace!("header::Extract({:?})", self.name);
        route::with(|route| {
            route.headers()
                .get(self.name)
                .and_then(|val| {
                    val.to_str().ok()
                })
                .and_then(|s| {
                    T::from_str(s).ok()
                })
        })
    }
}

impl<T: FromStr> FilterAnd for Extract<T> {}
