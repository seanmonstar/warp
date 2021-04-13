use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::pin_project;

use super::{Filter, FilterBase, Internal};

#[derive(Clone, Copy, Debug)]
pub struct TupleAll<F> {
    pub(super) filter: F,
}

impl<F> FilterBase for TupleAll<F>
where
    F: Filter,
{
    type Extract = (F::Extract,);
    type Error = F::Error;
    type Future = TupleAllFuture<F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        TupleAllFuture {
            extract: self.filter.filter(Internal),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct TupleAllFuture<F: Filter> {
    #[pin]
    extract: F::Future,
}

impl<F> Future for TupleAllFuture<F>
where
    F: Filter,
{
    type Output = Result<(F::Extract,), F::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match ready!(self.project().extract.try_poll(cx)) {
            Ok(args) => Poll::Ready(Ok((args,))),
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
