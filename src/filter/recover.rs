use std::mem;

use futures::{Async, Future, IntoFuture, Poll};

use super::{Filter, FilterBase, Func};
use generic::Either;
use route;

#[derive(Clone, Copy, Debug)]
pub struct Recover<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for Recover<T, F>
where
    T: Filter,
    F: Func<T::Error> + Clone + Send,
    F::Output: IntoFuture<Error = T::Error> + Send,
    <F::Output as IntoFuture>::Future: Send,
{
    type Extract = (Either<T::Extract, (<F::Output as IntoFuture>::Item,)>,);
    type Error = <F::Output as IntoFuture>::Error;
    type Future = RecoverFuture<T, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        let idx = route::with(|route| route.matched_path_index());
        RecoverFuture {
            state: State::First(self.filter.filter(), self.callback.clone()),
            original_path_index: PathIndex(idx),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct RecoverFuture<T: Filter, F>
where
    T: Filter,
    F: Func<T::Error>,
    F::Output: IntoFuture<Error = T::Error> + Send,
    <F::Output as IntoFuture>::Future: Send,
{
    state: State<T, F>,
    original_path_index: PathIndex,
}

enum State<T, F>
where
    T: Filter,
    F: Func<T::Error>,
    F::Output: IntoFuture<Error = T::Error> + Send,
    <F::Output as IntoFuture>::Future: Send,
{
    First(T::Future, F),
    Second(<F::Output as IntoFuture>::Future),
    Done,
}

struct PathIndex(usize);

impl PathIndex {
    fn reset_path(&self) {
        route::with(|route| route.reset_matched_path_index(self.0));
    }
}

impl<T, F> Future for RecoverFuture<T, F>
where
    T: Filter,
    F: Func<T::Error>,
    F::Output: IntoFuture<Error = T::Error> + Send,
    <F::Output as IntoFuture>::Future: Send,
{
    type Item = (Either<T::Extract, (<F::Output as IntoFuture>::Item,)>,);
    type Error = <F::Output as IntoFuture>::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let err = match self.state {
            State::First(ref mut first, _) => match first.poll() {
                Ok(Async::Ready(ex)) => return Ok(Async::Ready((Either::A(ex),))),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => err,
            },
            State::Second(ref mut second) => {
                return match second.poll() {
                    Ok(Async::Ready(ex2)) => Ok(Async::Ready((Either::B((ex2,)),))),
                    Ok(Async::NotReady) => Ok(Async::NotReady),
                    Err(e) => Err(e),
                };
            }
            State::Done => panic!("polled after complete"),
        };

        self.original_path_index.reset_path();

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.call(err).into_future(),
            _ => unreachable!(),
        };

        match second.poll()? {
            Async::Ready(item) => Ok(Async::Ready((Either::B((item,)),))),
            Async::NotReady => {
                self.state = State::Second(second);
                Ok(Async::NotReady)
            }
        }
    }
}
