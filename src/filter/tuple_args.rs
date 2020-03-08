use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::pin_project;

use super::{Filter, FilterBase, Internal};

#[derive(Clone, Copy, Debug)]
pub struct TupleArgs<F> {
    pub(super) filter: F,
}

impl<F> FilterBase for TupleArgs<F>
where
    F: Filter,
{
    type Extract = (F::Extract,);
    type Error = F::Error;
    type Future = TupleArgsFuture<F>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        TupleArgsFuture {
            extract: self.filter.filter(Internal),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct TupleArgsFuture<F: Filter> {
    #[pin]
    extract: F::Future,
}

impl<F> Future for TupleArgsFuture<F>
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
