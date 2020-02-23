//! Request Extensions

use std::convert::Infallible;

use futures::future;

use crate::filter::{filter_fn_one, Filter, WrapSealed};
use crate::reject::{self, Rejection};

use self::internal::WithExtensions_;

/// Get a previously set extension of the current route.
///
/// If the extension doesn't exist, this rejects with a `MissingExtension`.
pub fn get<T: Clone + Send + Sync + 'static>(
) -> impl Filter<Extract = (T,), Error = Rejection> + Copy {
    filter_fn_one(|route| {
        let route = route
            .extensions()
            .get::<T>()
            .cloned()
            .ok_or_else(|| reject::known(MissingExtension { _p: () }));
        future::ready(route)
    })
}

/// Get a previously set extension of the current route.
///
/// If the extension doesn't exist, it yields `None`.
pub fn optional<T: Clone + Send + Sync + 'static>(
) -> impl Filter<Extract = (Option<T>,), Error = Infallible> + Copy {
    filter_fn_one(|route| future::ok(route.extensions().get::<T>().cloned()))
}

unit_error! {
    /// An error used to reject if `get` cannot find the extension.
    pub MissingExtension: "Missing request extension"
}

/// Access to request `Extensions`.
/// A given function is called before reaching a wrapped filter.
pub fn with_mut<F>(f: F) -> WithExtensions<F>
where
    F: Fn(&mut http::Extensions),
{
    WithExtensions { f }
}

/// Decorates a `Filter` to access `http::Extensions`.
#[derive(Clone, Copy, Debug)]
pub struct WithExtensions<F> {
    f: F,
}

impl<FN, F> WrapSealed<F> for WithExtensions<FN>
where
    FN: Fn(&mut http::Extensions) + Clone + Send,
    F: Filter + Clone + Send,
{
    type Wrapped = WithExtensions_<FN, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        WithExtensions_ {
            f: self.f.clone(),
            filter,
        }
    }
}

mod internal {
    #[allow(missing_debug_implementations)]
    pub struct WithExtensions_<FN, F> {
        pub(super) f: FN,
        pub(super) filter: F,
    }

    impl<FN, F> FilterBase for WithExtensions_<FN, F>
    where
        FN: Fn(&mut http::Extensions) + Clone + Send,
        F: Filter,
    {
        type Extract = F::Extract;
        type Error = F::Error;
        type Future = WithExtensionsFuture<FN, F::Future>;

        fn filter(&self, _: Internal) -> Self::Future {
            WithExtensionsFuture {
                f: self.f.clone(),
                future: self.filter.filter(Internal),
            }
        }
    }

    use crate::filter::{Filter, FilterBase, Internal};
    use crate::route;
    use pin_project::pin_project;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[allow(missing_debug_implementations)]
    #[pin_project]
    pub struct WithExtensionsFuture<FN, F> {
        f: FN,
        #[pin]
        future: F,
    }

    impl<FN, F> Future for WithExtensionsFuture<FN, F>
    where
        F: Future,
        FN: Fn(&mut http::Extensions),
    {
        type Output = F::Output;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let pin = self.as_mut().project();
            route::with(|route| (pin.f)(route.extensions_mut()));
            pin.future.poll(cx)
        }
    }
}
