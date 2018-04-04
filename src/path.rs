use std::marker::PhantomData;
use std::str::FromStr;

use http::Uri;

use ::filter::{Filter};
use ::Request;

pub fn path<T>() -> Extract<T> {
    Extract {
        _marker: PhantomData,
    }
}

pub fn exact(p: &'static str) -> Const {
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
    _marker: PhantomData<T>,
}

#[derive(Clone, Copy, Debug)]
pub struct Const {
    p: &'static str,
}

impl<T> Filter for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter(&self, input: &mut Request) -> Option<T> {
        trace!("filter::Extract: {:?}", input.uri().path());
        let mut path_segs = input.uri().path().split('/');
        if let Some(seg) = path_segs.next() {
            // Should hopefully be empty
            debug_assert!(seg.is_empty());
        }
        path_segs.next()
            .and_then(|seg| {
                T::from_str(seg).ok()
            })
    }
}

impl Filter for Const {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> Option<()> {
        trace!("filter::Const({:?}): {:?}", self.p, input.uri().path());
        if input.uri().path().contains(self.p) {
            *input.uri_mut() = {
                if self.p.len() == input.uri().path().len() {
                    Uri::default()
                } else {
                    Uri::from_str(&input.uri().path()[self.p.len()..])
                        .expect("unimplemented")
                }
            };
            Some(())
        } else {
            None
        }
    }
}

impl Filter for &'static str {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> Option<()> {
        Const {
            p: self,
        }.filter(input)
    }
}

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
