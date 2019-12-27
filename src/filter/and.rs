use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;
use pin_project::{pin_project, project};

use super::{Combine, Filter, FilterBase, HList, Internal, Tuple};
use crate::reject::CombineRejection;

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<T, U> FilterBase for And<T, U>
where
    T: Filter,
    T::Extract: Send,
    U: Filter + Clone + Send,
    <T::Extract as Tuple>::HList: Combine<<U::Extract as Tuple>::HList> + Send,
    <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple: Send,
    U::Error: CombineRejection<T::Error>,
{
    type Extract = <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple;
    type Error = <U::Error as CombineRejection<T::Error>>::One;
    type Future = AndFuture<T, U>;

    fn filter(&self, _: Internal) -> Self::Future {
        AndFuture {
            state: State::First(self.first.filter(Internal), self.second.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndFuture<T: Filter, U: Filter> {
    #[pin]
    state: State<T, U>,
}

#[pin_project]
enum State<T: Filter, U: Filter> {
    First(#[pin] T::Future, U),
    Second(Option<T::Extract>, #[pin] U::Future),
    Done,
}

impl<T, U> Future for AndFuture<T, U>
where
    T: Filter,
    U: Filter,
    <T::Extract as Tuple>::HList: Combine<<U::Extract as Tuple>::HList> + Send,
    U::Error: CombineRejection<T::Error>,
{
    type Output = Result<
            <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple,
        <U::Error as CombineRejection<T::Error>>::One>;

    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let (ex1, fut2) = match pin.state.project() {
                State::First(first, second) => match ready!(first.poll(cx)) {
                    Ok(first) => (first, second.filter(Internal)),
                    Err(err) => return Poll::Ready(Err(From::from(err))),
                },
                State::Second(ex1, second) => {
                    let ex2 = match ready!(second.poll(cx)) {
                        Ok(second) => second,
                        Err(err) => return Poll::Ready(Err(From::from(err))),
                    };
                    let ex3 = ex1.take().unwrap().hlist().combine(ex2.hlist()).flatten();
                    self.set(AndFuture { state: State::Done });
                    return Poll::Ready(Ok(ex3));
                }
                State::Done => panic!("polled after complete"),
            };

            self.set(AndFuture {
                state: State::Second(Some(ex1), fut2),
            });
        }
    }
}
