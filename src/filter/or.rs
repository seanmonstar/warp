use std::mem;

use futures::{Async, Future, Poll};

use ::error::CombineError;
use ::route::Route;
use super::{Cons, cons, Extracted, Errored, FilterBase, Filter};

#[derive(Clone, Copy, Debug)]
pub struct Or<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

#[derive(Debug)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

impl<T, U> FilterBase for Or<T, U>
where
    T: Filter,
    U: Filter + Clone + Send,
    U::Error: CombineError<T::Error>,
{
    type Extract = Cons<
        Either<
            T::Extract,
            U::Extract,
        >
    >;
    type Error = <U::Error as CombineError<T::Error>>::Error;
    type Future = EitherFuture<T, U>;

    fn filter(&self, route: Route) -> Self::Future {
        let idx = route.matched_path_index();
        EitherFuture {
            state: State::First(self.first.filter(route), self.second.clone()),
            original_path_index: idx,
        }
    }
}

pub struct EitherFuture<T: Filter, U: Filter> {
    state: State<T, U>,
    original_path_index: usize,
}

enum State<T: Filter, U: Filter> {
    First(T::Future, U),
    Second(U::Future),
    Done,
}

/*
impl<T, U> EitherFuture<T, U>
where
    U: Filter,
{
    fn reset_path(&self) {
        route::with(|route| {
            route.reset_matched_path_index(self.original_path_index)
        });
    }
}
*/

impl<T, U> Future for EitherFuture<T, U>
where
    T: Filter,
    U: Filter,
    U::Error: CombineError<T::Error>,
{
    type Item = Extracted<Cons<Either<T::Extract, U::Extract>>>;
    type Error = Errored<<U::Error as CombineError<T::Error>>::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let Errored(mut route, _e) = match self.state {
            State::First(ref mut first, _) => match first.poll() {
                Ok(Async::Ready(ex1)) => {
                    return Ok(Async::Ready(Extracted(ex1.0, cons(Either::A(ex1.1)))));
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => e,
            },
            State::Second(ref mut second) => return match second.poll() {
                Ok(Async::Ready(ex2)) => Ok(Async::Ready(ex2.map(|ex| {
                    cons(Either::B(ex))
                }))),
                Ok(Async::NotReady) => Ok(Async::NotReady),

                Err(Errored(mut route, e)) => {
                    route.reset_matched_path_index(self.original_path_index);
                    Err(Errored(route, e.into()))
                }
            },
            State::Done => panic!("polled after complete"),
        };

        route.reset_matched_path_index(self.original_path_index);

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.filter(route),
            _ => unreachable!(),
        };

        match second.poll() {
            Ok(Async::Ready(ex2)) => {
                Ok(Async::Ready(ex2.map(|ex2| cons(Either::B(ex2)))))
            },
            Ok(Async::NotReady) => {
                self.state = State::Second(second);
                Ok(Async::NotReady)
            }
            Err(Errored(mut route, e)) => {
                route.reset_matched_path_index(self.original_path_index);
                return Err(Errored(route, e.into()));
            }
        }
    }
}
