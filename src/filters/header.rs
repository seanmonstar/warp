use std::marker::PhantomData;
use std::str::FromStr;

use ::filter::{Filter, FilterAnd};
use ::route::Route;

pub fn header<T>(name: &'static str) -> Extract<T> {
    Extract {
        name,
        _marker: PhantomData,
    }
}

pub fn exact(name: &'static str, value: &'static str) -> Exact {
    Exact {
        name,
        value,
    }
}

#[derive(Clone, Debug)]
pub struct Exact {
    name: &'static str,
    value: &'static str,
}

impl Filter for Exact {
    type Extract = ();

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, ())> {
        trace!("header::Exact({:?}, {:?})", self.name, self.value);
        route.headers()
            .get(self.name)
            .and_then(|val| {
                if val == self.value {
                    Some(())
                } else {
                    None
                }
            })
            .map(|()| (route, ()))
    }
}

impl FilterAnd for Exact {}

pub struct Extract<T> {
    name: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Filter for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, T)> {
        trace!("header::Extract({:?})", self.name);
        route.headers()
            .get(self.name)
            .and_then(|val| {
                val.to_str().ok()
            })
            .and_then(|s| {
                T::from_str(s).ok()
            })
            .map(|val| (route, val))
    }
}

impl<T: FromStr> FilterAnd for Extract<T> {}
