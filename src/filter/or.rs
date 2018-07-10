use std::mem;

use futures::{Async, Future, Poll};

use ::reject::CombineRejection;
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
    U::Error: CombineRejection<T::Error>,
{
    type Extract = Cons<
        Either<
            T::Extract,
            U::Extract,
        >
    >;
    type Error = <U::Error as CombineRejection<T::Error>>::Rejection;
    type Future = EitherFuture<T, U>;

    fn filter(&self) -> Self::Future {
        let idx = route::with(|route| route.matched_path_index());
        EitherFuture {
            state: State::First(self.first.filter(), self.second.clone()),
            original_path_index: PathIndex(idx),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct EitherFuture<T: Filter, U: Filter> {
    state: State<T, U>,
    original_path_index: PathIndex,
}

enum State<T: Filter, U: Filter> {
    First(T::Future, U),
    Second(U::Future),
    Done,
}

struct PathIndex(usize);

impl PathIndex {
    fn reset_path(&self) {
        route::with(|route| {
            route.reset_matched_path_index(self.0)
        });
    }
}

impl<T, U> Future for EitherFuture<T, U>
where
    T: Filter,
    U: Filter,
    U::Error: CombineRejection<T::Error>,
{
    type Item = Cons<Either<T::Extract, U::Extract>>;
    type Error = <U::Error as CombineRejection<T::Error>>::Rejection;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let _e = match self.state {
            State::First(ref mut first, _) => match first.poll() {
                Ok(Async::Ready(ex1)) => {
                    return Ok(Async::Ready(cons(Either::A(ex1))));
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => e,
            },
            State::Second(ref mut second) => return match second.poll() {
                Ok(Async::Ready(ex2)) => Ok(Async::Ready(cons(Either::B(ex2)))),
                Ok(Async::NotReady) => Ok(Async::NotReady),

                Err(e) => {
                    self.original_path_index.reset_path();
                    Err(e.into())
                }
            },
            State::Done => panic!("polled after complete"),
        };

        self.original_path_index.reset_path();

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
                self.original_path_index.reset_path();
                return Err(e.into());
            }
        }
    }
}

