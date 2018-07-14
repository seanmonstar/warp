use std::mem;

use futures::{Async, Future, IntoFuture, Poll};

use ::reject::CombineRejection;
use super::{FilterBase, Filter, Func, HList, One, one};

#[derive(Clone, Copy, Debug)]
pub struct AndThen<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for AndThen<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple> + Clone + Send,
    F::Output: IntoFuture + Send,
    <F::Output as IntoFuture>::Error: CombineRejection<T::Error>,
    <F::Output as IntoFuture>::Future: Send,
{
    type Extract = One<<F::Output as IntoFuture>::Item>;
    type Error = <<F::Output as IntoFuture>::Error as CombineRejection<T::Error>>::Rejection;
    type Future = AndThenFuture<T, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        AndThenFuture {
            state: State::First(self.filter.filter(), self.callback.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct AndThenFuture<T: Filter, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
    F::Output: IntoFuture + Send,
    <F::Output as IntoFuture>::Error: CombineRejection<T::Error>,
    <F::Output as IntoFuture>::Future: Send,
{
    state: State<T, F>,
}

enum State<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
    F::Output: IntoFuture + Send,
    <F::Output as IntoFuture>::Error: CombineRejection<T::Error>,
    <F::Output as IntoFuture>::Future: Send,
{
    First(T::Future, F),
    Second(<F::Output as IntoFuture>::Future),
    Done,
}

impl<T, F> Future for AndThenFuture<T, F>
where
    T: Filter,
    T::Extract: HList,
    F: Func<<T::Extract as HList>::Tuple>,
    F::Output: IntoFuture + Send,
    <F::Output as IntoFuture>::Error: CombineRejection<T::Error>,
    <F::Output as IntoFuture>::Future: Send,
{
    type Item = One<<F::Output as IntoFuture>::Item>;
    type Error = <<F::Output as IntoFuture>::Error as CombineRejection<T::Error>>::Rejection;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let ex1 = match self.state {
            State::First(ref mut first, _) => {
                try_ready!(first.poll())
            },
            State::Second(ref mut second) => {
                let item = try_ready!(second.poll());
                return Ok(Async::Ready(one(item)));
            },
            State::Done => panic!("polled after complete"),
        };

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.call(ex1.flatten()).into_future(),
            _ => unreachable!(),
        };

        match second.poll()? {
            Async::Ready(item) => {
                Ok(Async::Ready(one(item)))
            },
            Async::NotReady => {
                self.state = State::Second(second);
                Ok(Async::NotReady)
            },
        }

    }
}

