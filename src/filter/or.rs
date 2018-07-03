use std::mem;

use futures::{Async, Future, Poll};

use ::error::CombineError;
use ::route;
use super::{Cons, cons, FilterBase, Filter};

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
    type Future = EitherFuture<T::Future, U>;

    fn filter(&self) -> Self::Future {
        let idx = route::with(|route| {
            route.matched_path_index()
        });
        EitherFuture {
            state: State::First(self.first.filter(), self.second.clone()),
            original_path_index: idx,
        }
        /*
        route::with(|route| {
            route
                .transaction(|| {
                    self.first.filter()
                })
                .map(Either::A)
                .or_else(|| {
                    route.transaction(|| {
                        self
                            .second
                            .filter()
                            .map(Either::B)
                    })
                })
                .map(|e| HCons(e, ()))
        })
        */
    }
}

pub struct EitherFuture<T, U: Filter> {
    state: State<T, U>,
    original_path_index: usize,
}

enum State<T, U: Filter> {
    First(T, U),
    Second(U::Future),
    Done,
}

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

impl<T, U> Future for EitherFuture<T, U>
where
    T: Future,
    U: Filter,
    U::Error: CombineError<T::Error>,
{
    type Item = Cons<Either<T::Item, U::Extract>>;
    type Error = <U::Error as CombineError<T::Error>>::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.state {
            State::First(ref mut first, _) => match first.poll() {
                Ok(Async::Ready(ex1)) => {
                    return Ok(Async::Ready(cons(Either::A(ex1))));
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_e) => {},
            },
            State::Second(ref mut second) => return match second.poll() {
                Ok(Async::Ready(ex2)) => Ok(Async::Ready(cons(Either::B(ex2)))),
                Ok(Async::NotReady) => Ok(Async::NotReady),
                Err(e) => {
                    let idx = self.original_path_index;
                    route::with(|route| {
                        route.reset_matched_path_index(idx)
                    });
                    Err(e.into())
                }
            },
            State::Done => panic!("polled after complete"),
        };

        self.reset_path();

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.filter(),
            _ => unreachable!(),
        };

        match second.poll() {
            Ok(Async::Ready(ex2)) => {
                Ok(Async::Ready(cons(Either::B(ex2))))
            },
            Ok(Async::NotReady) => {
                self.state = State::Second(second);
                Ok(Async::NotReady)
            }
            Err(e) => {
                self.reset_path();
                return Err(e.into());
            }
        }
    }
}
