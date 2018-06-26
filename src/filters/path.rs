//! dox?
use std::marker::PhantomData;
use std::str::FromStr;

use ::filter::{FilterBase, FilterAnd};
use ::route;

/// Create an `Extract` path filter.
///
/// An `Extract` will try to parse a value from the current request path
/// segment, and if successful, the value is returned as the `Filter`'s
/// "extracted" value.
pub fn path<T>() -> Extract<T> {
    Extract {
        _marker: PhantomData,
    }
}

/// Create an exact match path `Filter`.
///
/// This will try to match exactly to the current request path segment.
///
/// # Note
///
/// Exact path filters cannot be empty, or contain slashes.
pub fn exact(p: &'static str) -> Const {
    assert!(!p.is_empty(), "exact path segments should not be empty");
    assert!(!p.contains('/'), "exact path segments should not contain a slash: {:?}", p);
    Const {
        p,
    }
}

/*
pub fn index() -> Const {
    Const {
        p: "/",
    }
}
*/

/// dox?
pub struct Extract<T> {
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Extract<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Extract<T> {}

/// dox?
#[derive(Clone, Copy, Debug)]
pub struct Const {
    p: &'static str,
}

impl<T> FilterBase for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter(&self) -> Option<T> {
        route::with(|route| {
            route.filter_segment(|seg| {
                trace!("extract?: {:?}", seg);
                T::from_str(seg).ok()
            })
        })
    }
}

impl<T: FromStr> FilterAnd for Extract<T> {}

impl FilterBase for Const {
    type Extract = ();

    fn filter(&self) -> Option<()> {
        route::with(|route| {
            route.filter_segment(|seg| {
                trace!("({:?})?: {:?}", self.p, seg);
                if seg == self.p {
                    Some(())
                } else {
                    None
                }
            })
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
