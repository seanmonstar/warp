use std::marker::PhantomData;
use std::ops::Div;
use std::str::FromStr;

use http::Uri;

use ::filter::{And, Filter, FilterResult};
use ::Request;

pub fn path<T>() -> Extract<T> {
    Extract {
        _marker: PhantomData,
    }
}

pub struct Extract<T> {
    _marker: PhantomData<T>,
}

/*
pub struct Paths<T, U> {
    left: T,
    right: U,
}
*/

pub struct Const {
    p: &'static str,
}

/*
impl<T, U> Filter<Uri> for Paths<T, U>
where
    T: Filter<Uri>,
    U: Filter<Uri>,
{
    type Extract = (T::Extract, U::Extract);

    fn filter(&self, input: Uri) -> FilterResult<Uri, Self::Extract> {
        match self.left.filter(input).inner {
            FilterRes::Matched(input, extract1) => {
                match self.right.filter(input).inner {
                    FilterRes::Matched(input, extract2) => {
                        FilterResult::matched(input, (extract1, extract2))
                    },
                    FilterRes::Skipped(input) => FilterResult::skipped(input),
                }
            },
            FilterRes::Skipped(input) => FilterResult::skipped(input),
        }
    }
}
*/

impl<T> Filter for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter(&self, input: &mut Request) -> FilterResult<T> {
        trace!("filter::Extract: {:?}", input.uri().path());
        let extracted = {
            let mut path_segs = input.uri().path().split('/');
            if let Some(seg) = path_segs.next() {
                // Should hopefully be empty
                debug_assert!(seg.is_empty());
            }
            path_segs.next()
                .and_then(|seg| {
                    T::from_str(seg).ok()
                })
        };

        match extracted {
            Some(extracted) => FilterResult::Matched(extracted),
            None => FilterResult::Skipped,
        }
    }
}

impl Filter for Const {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> FilterResult<()> {
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
            FilterResult::Matched(())
        } else {
            FilterResult::Skipped
        }
    }
}

impl Filter for &'static str {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> FilterResult<()> {
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
