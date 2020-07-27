use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, TryFuture};
use pin_project::pin_project;

use super::{Filter, FilterBase, Func, Internal};

#[derive(Clone, Copy, Debug)]
pub struct Inject<T, V> {
    pub(super) filter: T,
    pub(super) variable: V,
}

impl<T, V> FilterBase for Inject<T, V>
where
    T: Filter,
    V: Clone + Send,
{
    type Extract = (V, T::Extract,);
    type Error = T::Error;
    type Future = InjectFuture<T, V>;
    #[inline]
    fn filter(&self, _: Internal) -> Self::Future {
        InjectFuture {
            extract: self.filter.filter(Internal),
            variable: self.variable.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct InjectFuture<T: Filter, V> {
    #[pin]
    extract: T::Future,
    variable: V,
}

impl<T, V> Future for InjectFuture<T, V>
where
    T: Filter,
    V: Clone + Send,
{
    type Output = Result<(V, T::Extract,), T::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match ready!(pin.extract.try_poll(cx)) {
            Ok(ex) => {
                let ex = (pin.variable.clone(), ex,);
                Poll::Ready(Ok(ex))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
