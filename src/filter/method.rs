use std::ops::Add;
use http;

use ::filter::{And, Filter, FilterResult};
use ::Request;

pub fn method(m: http::Method) -> Method {
    Method {
        m,
    }
}

pub const GET: Method = Method { m: http::Method::GET };
pub const POST: Method = Method { m: http::Method::POST };
pub const PUT: Method = Method { m: http::Method::PUT };
pub const DELETE: Method = Method { m: http::Method::DELETE };


pub struct Method {
    m: http::Method,
}

impl Filter for Method {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> FilterResult<()> {
        trace!("filter({:?}): {:?}", self.m, input.method());
        if &self.m == input.method() {
            FilterResult::Matched(())
        } else {
            FilterResult::Skipped
        }
    }
}

pub struct MethodAnd<T> {
    m: Method,
    t: T,
}

impl<T> Filter for MethodAnd<T>
where
    T: Filter,
{
    type Extract = T::Extract;

    fn filter(&self, input: &mut Request) -> FilterResult<Self::Extract> {
        match self.m.filter(input) {
            FilterResult::Matched(()) => {
                match self.t.filter(input) {
                    FilterResult::Matched(extract) => {
                        FilterResult::Matched(extract)
                    },
                    FilterResult::Skipped => FilterResult::Skipped,
                }
            },
            FilterResult::Skipped => FilterResult::Skipped,
        }

    }
}

impl<T> Add<T> for Method
where
    T: Filter,
{
    type Output = MethodAnd<T>;

    fn add(self, rhs: T) -> Self::Output {
        MethodAnd {
            m: self,
            t: rhs,
        }
    }
}
