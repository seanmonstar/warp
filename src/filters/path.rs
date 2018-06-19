use std::marker::PhantomData;
use std::str::FromStr;

use ::filter::{FilterBase, FilterAnd};
use ::route::Route;

pub fn path<T>() -> Extract<T> {
    Extract {
        _marker: PhantomData,
    }
}

pub fn exact(p: &'static str) -> Const {
    assert!(!p.is_empty(), "exact path segments should not be empty");
    assert!(!p.contains('/'), "exact path segments should not contain a slash: {:?}", p);
    Const {
        p,
    }
}

pub fn index() -> Const {
    Const {
        p: "/",
    }
}

pub struct Extract<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Extract<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Extract<T> {}

#[derive(Clone, Copy, Debug)]
pub struct Const {
    p: &'static str,
}

impl<T> FilterBase for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, T)> {
        route.filter_segment(|seg| {
            trace!("extract?: {:?}", seg);
            T::from_str(seg).ok()
        })
    }
}

impl<T: FromStr> FilterAnd for Extract<T> {}

impl FilterBase for Const {
    type Extract = ();

    fn filter<'a>(&self, route: Route<'a>) -> Option<(Route<'a>, ())> {
        route.filter_segment(|seg| {
            trace!("({:?})?: {:?}", self.p, seg);
            if seg == self.p {
                Some(())
            } else {
                None
            }
        })
    }
}

impl FilterAnd for Const {}

// Silly operator overloads...
//
// Like, filtering a "{username}/{id}" could be:
//
// extract::<&str>() / extract::<u64>

/*
impl<T> Div<Extract<T>> for &'static str
where
    T: FromStr,
{
    type Output = And<Const, Extract<T>>;

    fn div(self, rhs: Extract<T>) -> Self::Output {
        Const {
            p: self,
        }.and(rhs)
    }
}
*/
