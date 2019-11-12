use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

use pin_project::pin_project;
use futures::TryFuture;

use super::{Filter, FilterBase};
use crate::reject::IsReject;

#[derive(Clone, Copy, Debug)]
pub struct MapErr<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, E> FilterBase for MapErr<T, F>
where
    T: Filter,
    F: Fn(T::Error) -> E + Clone + Send,
    E: IsReject,
{
    type Extract = T::Extract;
    type Error = E;
    type Future = MapErrFuture<T, F>;
    #[inline]
    fn filter(&self) -> Self::Future {
        MapErrFuture {
            extract: self.filter.filter(),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct MapErrFuture<T: Filter, F> {
    #[pin]
    extract: T::Future,
    callback: F,
}

impl<T, F, E> Future for MapErrFuture<T, F>
where
    T: Filter,
    F: Fn(T::Error) -> E,
{
    type Output = Result<T::Extract, E>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        self.as_mut().project().extract.try_poll(cx).map_err(|err| (self.callback)(err))
    }
}
