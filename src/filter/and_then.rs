use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::{pin_project, project};

use super::{Filter, FilterBase, Func, Internal};
use crate::reject::CombineRejection;

#[derive(Clone, Copy, Debug)]
pub struct AndThen<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for AndThen<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone + Send,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: CombineRejection<T::Error>,
{
    type Extract = (<F::Output as TryFuture>::Ok,);
    type Error = <<F::Output as TryFuture>::Error as CombineRejection<T::Error>>::One;
    type Future = AndThenFuture<T, F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        AndThenFuture {
            state: State::First(self.filter.filter(Internal), self.callback.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndThenFuture<T: Filter, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: CombineRejection<T::Error>,
{
    #[pin]
    state: State<T, F>,
}

#[pin_project]
enum State<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: CombineRejection<T::Error>,
{
    First(#[pin] T::Future, F),
    Second(#[pin] F::Output),
    Done,
}

impl<T, F> Future for AndThenFuture<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: CombineRejection<T::Error>,
{
    type Output = Result<
        (<F::Output as TryFuture>::Ok,),
        <<F::Output as TryFuture>::Error as CombineRejection<T::Error>>::One,
    >;

    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let (ex1, second) = match pin.state.project() {
                State::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(first) => (first, second),
                    Err(err) => return Poll::Ready(Err(From::from(err))),
                },
                State::Second(second) => {
                    let ex3 = match ready!(second.try_poll(cx)) {
                        Ok(item) => Ok((item,)),
                        Err(err) => Err(From::from(err)),
                    };
                    self.set(AndThenFuture { state: State::Done });
                    return Poll::Ready(ex3);
                }
                State::Done => panic!("polled after complete"),
            };
            let fut2 = second.call(ex1);
            self.set(AndThenFuture {
                state: State::Second(fut2),
            });
        }
    }
}
