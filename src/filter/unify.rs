use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

use pin_project::pin_project;
use futures::{ready, TryFuture};

use super::{Either, Filter, FilterBase, Tuple};

#[derive(Clone, Copy, Debug)]
pub struct Unify<F> {
    pub(super) filter: F,
}

impl<F, T> FilterBase for Unify<F>
where
    F: Filter<Extract = (Either<T, T>,)>,
    T: Tuple,
{
    type Extract = T;
    type Error = F::Error;
    type Future = UnifyFuture<F::Future>;
    #[inline]
    fn filter(&self) -> Self::Future {
        UnifyFuture {
            inner: self.filter.filter(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct UnifyFuture<F> {
    #[pin]
    inner: F,
}

impl<F, T> Future for UnifyFuture<F>
where
    F: TryFuture<Ok = (Either<T, T>,)>,
{
    type Output = Result<T, F::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let unified = match ready!(self.project().inner.try_poll(cx)) {
            Ok((Either::A(a),)) => Ok(a),
            Ok((Either::B(b),)) => Ok(b),
            Err(err) => Err(err)
        };
        Poll::Ready(unified)
    }
}
