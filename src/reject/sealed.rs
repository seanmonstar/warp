use super::{Reason, Rejection};
use http::StatusCode;
use std::convert::Infallible;
use std::fmt;

// This sealed trait exists to allow Filters to return either `Rejection`
// or `!`. There are no other types that make sense, and so it is sealed.
pub trait IsReject: fmt::Debug + Send + Sync {
    fn status(&self) -> StatusCode;
    fn into_response(self) -> crate::reply::Response;
}

fn _assert_object_safe() {
    fn _assert(_: &dyn IsReject) {}
}

// This weird trait is to allow optimizations of propagating when a
// rejection can *never* happen (currently with the `Never` type,
// eventually to be replaced with `!`).
//
// Using this trait means the `Never` gets propagated to chained filters,
// allowing LLVM to eliminate more code paths. Without it, such as just
// requiring that `Rejection::from(Never)` were used in those filters,
// would mean that links later in the chain may assume a rejection *could*
// happen, and no longer eliminate those branches.
pub trait CombineRejection<E>: Send + Sized {
    /// The type that should be returned when only 1 of the two
    /// "rejections" occurs.
    ///
    /// # For example:
    ///
    /// `warp::any().and(warp::path("foo"))` has the following steps:
    ///
    /// 1. Since this is `and`, only **one** of the rejections will occur,
    ///    and as soon as it does, it will be returned.
    /// 2. `warp::any()` rejects with `Never`. So, it will never return `Never`.
    /// 3. `warp::path()` rejects with `Rejection`. It may return `Rejection`.
    ///
    /// Thus, if the above filter rejects, it will definitely be `Rejection`.
    type One: IsReject + From<Self> + From<E> + Into<Rejection>;

    /// The type that should be returned when both rejections occur,
    /// and need to be combined.
    type Combined: IsReject;

    fn combine(self, other: E) -> Self::Combined;
}

impl CombineRejection<Rejection> for Rejection {
    type One = Rejection;
    type Combined = Rejection;

    fn combine(self, other: Rejection) -> Self::Combined {
        let reason = match (self.reason, other.reason) {
          // If both are mismatches, add together their mismatches
          // Using a vector may take a bit longer, but copying may well take less
          // time than allocating a new Box would.
          (Reason::Mismatch(mut this), Reason::Mismatch(mut that)) => {
              this.append(&mut that);
              Reason::Mismatch(this)
          },
          // There should never occur two fatal errors, since the first should cause return
          // As such the implied priority of self should never matter.
          (Reason::Fatal(err), _) | (_, Reason::Fatal(err)) => Reason::Fatal(err),
        };
        Rejection{ reason }
    }
}

impl CombineRejection<Infallible> for Rejection {
    type One = Rejection;
    type Combined = Infallible;

    fn combine(self, other: Infallible) -> Self::Combined {
        match other {}
    }
}

impl CombineRejection<Rejection> for Infallible {
    type One = Rejection;
    type Combined = Infallible;

    fn combine(self, _: Rejection) -> Self::Combined {
        match self {}
    }
}

impl CombineRejection<Infallible> for Infallible {
    type One = Infallible;
    type Combined = Infallible;

    fn combine(self, _: Infallible) -> Self::Combined {
        match self {}
    }
}
