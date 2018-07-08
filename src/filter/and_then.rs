use std::mem;

use futures::{Async, Future, IntoFuture, Poll};

use ::reject::CombineRejection;
use ::route::Route;
use super::{Cons, Extracted, Errored, FilterBase, Filter, Func, cons, HList};

#[derive(Clone, Copy)]
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
    type Extract = Cons<<F::Output as IntoFuture>::Item>;
    type Error = <<F::Output as IntoFuture>::Error as CombineRejection<T::Error>>::Rejection;
    type Future = AndThenFuture<T, F>;
    #[inline]
    fn filter(&self, route: Route) -> Self::Future {
        AndThenFuture {
            state: State::First(self.filter.filter(route), self.callback.clone()),
        }
    }
}

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
    Second(Option<Route>, <F::Output as IntoFuture>::Future),
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
    type Item = Extracted<Cons<<F::Output as IntoFuture>::Item>>;
    type Error = Errored<<<F::Output as IntoFuture>::Error as CombineRejection<T::Error>>::Rejection>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let Extracted(route, ex1) = match self.state {
            State::First(ref mut first, _) => {
                try_ready!(first.poll().map_err(Errored::combined::<<F::Output as IntoFuture>::Error>))
            },
            State::Second(ref mut route, ref mut second) => {
                return match second.poll() {
                    Ok(Async::Ready(item)) => {
                        let r = route
                            .take()
                            .expect("polled after complete");
                        Ok(Async::Ready(Extracted(r, cons(item))))
                    },
                    Ok(Async::NotReady) => Ok(Async::NotReady),
                    Err(err) => {
                        let r = route
                            .take()
                            .expect("polled after complete");
                        Err(Errored(r, err.into()))
                    },
                };
            },
            State::Done => panic!("polled after complete"),
        };

        let mut second = match mem::replace(&mut self.state, State::Done) {
            State::First(_, second) => second.call(ex1.flatten()).into_future(),
            _ => unreachable!(),
        };

        match second.poll() {
            Ok(Async::Ready(item)) => {
                Ok(Async::Ready(Extracted(route, cons(item))))
            },
            Ok(Async::NotReady) => {
                self.state = State::Second(Some(route), second);
                Ok(Async::NotReady)
            },
            Err(err) => Err(Errored(route, err.into())),
        }

    }
}

