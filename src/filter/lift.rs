use std::cell::RefCell;

use futures::{Future, Async, Poll};
use tower_service::Service as TowerService;

use ::filter::Filter;
use ::Request;
use ::reply::{Reply, Response, ReplySealed};
use ::reject::Reject;
use ::never::Never;
use ::route::{self, Route};

/// Wraps a `Filter` instance, implementing `tower_service::Service`.
#[derive(Debug)]
pub struct LiftService<F> {
  filter: F
}

pub fn lift<F: Filter>(filter: F) -> LiftService<F> {
  LiftService { filter }
}

impl<F> TowerService for LiftService<F>
where
  F: Filter + Send + Sync + 'static,
  <F::Future as Future>::Item: Reply,
  <F::Future as Future>::Error: Reject,
{
  type Request = Request;
  type Response = Response;
  type Error = Never;
  type Future = ResponseFuture<F::Future>;

  #[inline]
  fn poll_ready(&mut self) -> Result<Async<()>, Self::Error> {
    Ok(Async::Ready(()))
  }

  #[inline]
  fn call(&mut self, req: Self::Request) -> Self::Future {
    debug_assert!(!route::is_set(), "nested route::set calls");

    let route = Route::new(req);
    let future = route::set(&route, || self.filter.filter());

    ResponseFuture {
      route,
      future
    }
  }
}

#[derive(Debug)]
pub struct ResponseFuture<F> {
  future: F,
  route:  RefCell<Route>
}

impl<F> Future for ResponseFuture<F>
where
  F: Future,
  F::Item: Reply,
  F::Error: Reject,
{
  type Item = Response;
  type Error = Never;

  #[inline]
  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    debug_assert!(!route::is_set(), "nested route::set calls");

    let future = &mut self.future;
    match route::set(&self.route, || future.poll()) {
      Ok(Async::Ready(ok)) => Ok(Async::Ready(ok.into_response())),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(err) => Ok(Async::Ready(err.into_response())),
    }
  }
}

