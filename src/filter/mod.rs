use ::Request;
use ::reply::Reply;

mod and;
mod map;
mod or;
mod service;

pub use self::and::And;
pub use self::map::Map;
pub use self::or::{Either, Or};
pub use self::service::FilteredService;

pub trait Filter {

    type Extract;

    fn filter(&self, input: &mut Request) -> Option<Self::Extract>;

    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        And {
            first: self,
            second: other,
        }
    }

    fn or<F>(self, other: F) -> Or<Self, F>
    where
        Self: Sized,
        F: Filter,
    {
        Or {
            first: self,
            second: other,
        }
    }

    fn map<F, U>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Extract) -> U,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    fn service(self) -> FilteredService<Self>
    where
        Self: Sized,
        Self::Extract: Reply,
    {
        FilteredService {
            filter: self,
        }
    }
}

