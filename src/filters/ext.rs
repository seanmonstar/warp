//! Request Extensions

use std::convert::Infallible;

use futures_util::future;

use crate::{
    filter::{filter_fn_one, Filter, WrapSealed},
    reject::{self, IsReject, Rejection},
    Reply,
};

use self::internal::DataProvider;

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

/// Create a wrapping filter to set an arbitrary value in the current route extensions.
///
/// After setting the value, it can be retrieved in another filter by
/// use `get` with the same type.
///
pub fn provide<T>(data: T) -> Provider<T> {
    Provider { data }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Provider<T> {
    data: T,
}

impl<T, F> WrapSealed<F> for Provider<T>
where
    T: Clone + Send + Sync + 'static,
    F: Filter + Clone + Send,
    F::Extract: Reply,
    F::Error: IsReject,
{
    type Wrapped = DataProvider<T, F>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        DataProvider {
            data: self.data.clone(),
            next: filter,
        }
    }
}

mod internal {
    use crate::filter::{Filter, FilterBase, Internal};
    use crate::reject::IsReject;
    use crate::reply::Reply;

    #[allow(missing_debug_implementations)]
    #[derive(Clone)]
    pub struct DataProvider<T, F> {
        pub(super) data: T,
        pub(super) next: F,
    }

    impl<T, F> FilterBase for DataProvider<T, F>
    where
        T: Clone + Send + Sync + 'static,
        F: Filter + Clone + Send,
        F::Extract: Reply,
        F::Error: IsReject,
    {
        type Error = F::Error;
        type Extract = F::Extract;
        type Future = F::Future;

        fn filter(&self, _: Internal) -> Self::Future {
            let data = self.data.clone();
            crate::route::with(|route| {
                route.extensions_mut().insert(data);
            });

            self.next.filter(Internal)
        }
    }
}
