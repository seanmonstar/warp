use std::mem;

use futures::{Async, Future, Poll};

use super::{Filter, FilterBase};
use generic::Either;
use reject::CombineRejection;
use route;

#[derive(Clone, Copy, Debug)]
pub struct Or<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> FilterBase for Or<T, U>
where
    T: Filter,
    U: Filter + Clone + Send,
    U::Error: CombineRejection<T::Error>,
{
    type Extract = (Either<T::Extract, U::Extract>,);
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
    Second(Option<T::Error>, U::Future),
    Done,
}

struct PathIndex(usize);

impl PathIndex {
    fn reset_path(&self) {
        route::with(|route| route.reset_matched_path_index(self.0));
    }
}

impl<T, U> Future for EitherFuture<T, U>
where
    T: Filter,
    U: Filter,
    U::Error: CombineRejection<T::Error>,
{
    type Item = (Either<T::Extract, U::Extract>,);
    type Error = <U::Error as CombineRejection<T::Error>>::Rejection;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let err1 = match self.state {
            State::First(ref mut first, _) => match first.poll() {
                Ok(Async::Ready(ex1)) => {
                    return Ok(Async::Ready((Either::A(ex1),)));
                }
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => e,
            },
            State::Second(ref mut err1, ref mut second) => {
                return match second.poll() {
                    Ok(Async::Ready(ex2)) => Ok(Async::Ready((Either::B(ex2),))),
                    Ok(Async::NotReady) => Ok(Async::NotReady),

                    Err(e) => {
                        self.original_path_index.reset_path();
                        let err1 = err1.take().expect("polled after complete");
                        Err(e.combine(err1))
                    }
                };
            }
            State::Done => panic!("polled after complete"),
        };

        self.original_path_index.reset_path();

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.filter(),
            _ => unreachable!(),
        };

        match second.poll() {
            Ok(Async::Ready(ex2)) => Ok(Async::Ready((Either::B(ex2),))),
            Ok(Async::NotReady) => {
                self.state = State::Second(Some(err1), second);
                Ok(Async::NotReady)
            }
            Err(e) => {
                self.original_path_index.reset_path();
                return Err(e.combine(err1));
            }
        }
    }
}
