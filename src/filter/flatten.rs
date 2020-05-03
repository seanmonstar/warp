use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::{pin_project, project};

use super::{Filter, FilterBase, Func, Internal};

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
    <F::Output as Future>::Output: Filter<Error = T::Error>,
{
    type Extract = <<F::Output as Future>::Output as FilterBase>::Extract;
    type Error = T::Error;
    type Future = FlattenFuture<T, F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        FlattenFuture::EvalFirst {
            first_future: self.filter.filter(Internal),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub enum FlattenFuture<T: Filter, F>
    where 
        T: Filter,
        F: Func<T::Extract>,
        F::Output: Future,
        <F::Output as Future>::Output: Filter<Error = T::Error>,
{
    EvalFirst {
        #[pin]
        first_future: T::Future,
        callback: F,
    },
    EvalCallback {
        #[pin]
        callback_future: F::Output,
    },
    EvalSecond {
        #[pin]
        second_future: <<F::Output as Future>::Output as FilterBase>::Future,
    }
}

impl<T, F> Future for FlattenFuture<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone,
    F::Output: Future,
    <F::Output as Future>::Output: Filter<Error = T::Error>,
{
    type Output = Result<<<F::Output as Future>::Output as FilterBase>::Extract, <<F::Output as Future>::Output as FilterBase>::Error>;

    #[inline]
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project(); 
            #[project]
            match pin {
                FlattenFuture::EvalFirst{first_future, callback} => {
                    match ready!(first_future.poll(cx)) {
                        Ok(ex) => {
                            let callback_future = callback.call(ex);
                            self.set(FlattenFuture::EvalCallback{callback_future});
                        }
                        Err(e) => {
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                FlattenFuture::EvalCallback{callback_future} => {
                    let filter = ready!(callback_future.poll(cx));
                    let second_future = filter.filter(Internal);
                    self.set(FlattenFuture::EvalSecond{second_future});    
                }
                FlattenFuture::EvalSecond { second_future } => {
                    match ready!(second_future.try_poll(cx)) {
                        Ok(item) => return Poll::Ready(Ok(item)),
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
            }
        }
    }
}
