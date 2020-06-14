use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::pin_project;

use super::{Filter, FilterBase, Func, Internal};
use crate::reject::IsReject;

#[derive(Clone, Copy, Debug)]
pub struct Negate<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F> FilterBase for Negate<T, F>
where
    T: Filter,
    F: Func<T::Extract> + Clone + Send,
    F::Output: IsReject,
{
    type Extract = ();
    type Error = F::Output;
    type Future = NegateFuture<T, F>;

    fn filter(&self, _: Internal) -> Self::Future {
        NegateFuture {
            extract: self.filter.filter(Internal),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct NegateFuture<T: Filter, F> {
    #[pin]
    extract: T::Future,
    callback: F,
}

impl<T, F> Future for NegateFuture<T, F>
where
    T: Filter,
    F: Func<T::Extract>,
    F::Output: IsReject,
{
    type Output = Result<(), F::Output>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match ready!(pin.extract.try_poll(cx)) {
            Ok(ex) => {
                let ex = pin.callback.call(ex);
                Poll::Ready(Err(ex))
            }
            Err(_) => Poll::Ready(Ok(())),
        }
    }
}
