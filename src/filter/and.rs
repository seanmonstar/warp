use std::mem;

use futures::{Async, Future, Poll};

use ::error::CombineError;
use super::{Combine, FilterBase, Filter, HList};

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> FilterBase for And<T, U>
where
    T: Filter,
    U: Filter + Clone + Send,
    T::Extract: Combine<U::Extract> + Send,
    U::Error: CombineError<T::Error>,
{
    type Extract = <T::Extract as Combine<U::Extract>>::Output;
    type Error = <U::Error as CombineError<T::Error>>::Error;
    type Future = AndFuture<T::Future, U>;

    fn filter(&self) -> Self::Future {
        AndFuture {
            state: State::First(self.first.filter(), self.second.clone()),
        }
    }
}

pub struct AndFuture<T: Future, U: Filter> {
    state: State<T, U>,
}

enum State<T: Future, U: Filter> {
    First(T, U),
    Second(Option<T::Item>, U::Future),
    Done,
}

impl<T, U> Future for AndFuture<T, U>
where
    T: Future,
    U: Filter,
    T::Item: Combine<U::Extract>,
    U::Error: CombineError<T::Error>,
{
    type Item = <T::Item as Combine<U::Extract>>::Output;
    type Error = <U::Error as CombineError<T::Error>>::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let ex1 = match self.state {
            State::First(ref mut first, _) => {
                try_ready!(first.poll())
            },
            State::Second(ref mut ex1, ref mut second) => {
                let ex2 = try_ready!(second.poll());
                return Ok(Async::Ready(ex1.take().unwrap().combine(ex2)));
            },
            State::Done => panic!("polled after complete"),
        };

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.filter(),
            _ => unreachable!(),
        };

        match second.poll()? {
            Async::Ready(ex2) => {
                Ok(Async::Ready(ex1.combine(ex2)))
            },
            Async::NotReady => {
                self.state = State::Second(Some(ex1), second);
                Ok(Async::NotReady)
            }
        }
    }
}
