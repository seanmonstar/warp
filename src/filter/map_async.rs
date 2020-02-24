use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::{pin_project, project};

use super::{Filter, FilterBase, Func, Internal};

#[derive(Clone, Copy, Debug)]
pub struct MapAsync<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for MapAsync<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone + Send,
    F::Output: Future + Send,
{
    type Extract = (<F::Output as Future>::Output,);
    type Error = T::Error;
    type Future = MapAsyncFuture<T, F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        MapAsyncFuture {
            state: State::First(self.filter.filter(Internal), self.callback.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct MapAsyncFuture<T: Filter, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: Future + Send,
{
    #[pin]
    state: State<T, F>,
}

#[pin_project]
enum State<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: Future + Send,
{
    First(#[pin] T::Future, F),
    Second(#[pin] F::Output),
    Done,
}

impl<T, F> Future for MapAsyncFuture<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: Future + Send,
{
    type Output = Result<(<F::Output as Future>::Output,), T::Error>;

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
                    let ex3 = ready!(second.poll(cx));
                    self.set(MapAsyncFuture { state: State::Done });
                    return Poll::Ready(Ok((ex3,)));
                }
                State::Done => panic!("polled after complete"),
            };
            let fut2 = second.call(ex1);
            self.set(MapAsyncFuture {
                state: State::Second(fut2),
            });
        }
    }
}
