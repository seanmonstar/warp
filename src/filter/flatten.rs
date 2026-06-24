use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::{pin_project, project};

use super::{CombineRejection, Filter, FilterBase, Func, Internal};

#[derive(Clone, Copy, Debug)]
pub struct Flatten<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for Flatten<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone + Send,
    F::Output: Future + Send,
    <F::Output as Future>::Output: Filter,
    <<F::Output as Future>::Output as FilterBase>::Error: CombineRejection<T::Error>,
{
    type Extract = <<F::Output as Future>::Output as FilterBase>::Extract;
    type Error =
        <<<F::Output as Future>::Output as FilterBase>::Error as CombineRejection<T::Error>>::One;
    type Future = FlattenFuture<T, F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        FlattenFuture {
            state: State::First(self.filter.filter(Internal), self.callback.clone()),
        }
    }
}

#[pin_project]
enum State<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: Future,
    <F::Output as Future>::Output: Filter,
    <<F::Output as Future>::Output as FilterBase>::Error: CombineRejection<T::Error>,
{
    First(#[pin] T::Future, F),
    Second(#[pin] F::Output),
    Third(#[pin] <<F::Output as Future>::Output as FilterBase>::Future),
    Done,
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct FlattenFuture<T: Filter, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: Future,
    <F::Output as Future>::Output: Filter,
    <<F::Output as Future>::Output as FilterBase>::Error: CombineRejection<T::Error>,
{
    #[pin]
    state: State<T, F>,
}

impl<T, F> Future for FlattenFuture<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone,
    F::Output: Future,
    <F::Output as Future>::Output: Filter,
    <<F::Output as Future>::Output as FilterBase>::Error: CombineRejection<T::Error>,
{
    type Output = Result<
        <<F::Output as Future>::Output as FilterBase>::Extract,
        <<<F::Output as Future>::Output as FilterBase>::Error as CombineRejection<T::Error>>::One,
    >;

    #[inline]
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            match pin.state.project() {
                State::First(first, callback) => match ready!(first.poll(cx)) {
                    Ok(ex) => {
                        let second = callback.call(ex);
                        self.set(FlattenFuture {
                            state: State::Second(second),
                        });
                    }
                    Err(e) => {
                        return Poll::Ready(Err(From::from(e)));
                    }
                },
                State::Second(second) => {
                    let filter = ready!(second.poll(cx));
                    let third = filter.filter(Internal);
                    self.set(FlattenFuture {
                        state: State::Third(third),
                    });
                }
                State::Third(third) => match ready!(third.try_poll(cx)) {
                    Ok(item) => {
                        self.set(FlattenFuture { state: State::Done });
                        return Poll::Ready(Ok(item));
                    }
                    Err(e) => return Poll::Ready(Err(From::from(e))),
                },
                State::Done => panic!("polled after complete"),
            }
        }
    }
}
