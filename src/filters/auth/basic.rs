//! Basic Authentication Filters

use std::sync::Arc;

use http::{
    self,
    header::{self, HeaderValue},
};

use self::internal::AuthFilter;
use crate::filter::{Filter, WrapSealed};
use crate::reject::{CombineRejection, Rejection};
use crate::reply::Reply;
use std::collections::HashSet;

/// Create a wrapping filter that exposes [AUTHENTICATION][] behavior for a wrapped
/// filter.
///
/// [AUTHENTICATION]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let auth = warp::auth::basic()
///     .realm("access")
///     .allow("user", "1234");
///
/// let route = warp::any()
///     .map(warp::reply)
///     .with(auth);
/// ```
pub fn basic() -> Builder {
    Builder {
        realm: None,
        authorizations: HashSet::new(),
    }
}

/// A wrapping filter constructed via `warp::auth()`.
#[derive(Clone, Debug)]
pub struct Auth {
    config: Arc<Configured>,
}

/// A builder constructed via `warp::auth()`.
#[derive(Clone, Debug)]
pub struct Builder {
    realm: Option<String>,
    authorizations: HashSet<Vec<u8>>,
}

impl Builder {
    /// Sets the realm.
    pub fn realm(mut self, realm: impl Into<String>) -> Self {
        self.realm = Some(realm.into());
        self
    }

    /// Adds a user and password to the list of authorized credentials.
    pub fn allow(mut self, user: &str, pass: &str) -> Self {
        let authorization = base64::encode(format!("{}:{}", user, pass)).into();
        self.authorizations.insert(authorization);
        self
    }

    /// Builds the `Auth` wrapper from the configured settings.
    ///
    /// This step isn't *required*, as the `Builder` itself can be passed
    /// to `Filter::with`. This just allows constructing once, thus not needing
    /// to pay the cost of "building" every time.
    ///     
    /// # Panics
    ///
    /// Panics if the provided realm is not valid in a `http::header::HeaderValue`.
    pub fn build(self) -> Auth {
        let authenticate_header = match self.realm {
            Some(realm) => {
                HeaderValue::from_str(&format!("Basic realm=\"{}\", charset=\"UTF-8\"", realm))
                    .expect("illegal realm")
            }
            None => HeaderValue::from_static("Basic charset=\"UTF-8\""),
        };

        let config = Arc::new(Configured {
            authenticate_header,
            authorizations: self.authorizations,
        });
        Auth { config }
    }
}

impl<F> WrapSealed<F> for Builder
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::One: CombineRejection<Rejection>,
{
    type Wrapped = AuthFilter<F>;

    fn wrap(&self, inner: F) -> Self::Wrapped {
        let Auth { config } = self.clone().build();
        AuthFilter { config, inner }
    }
}

impl<F> WrapSealed<F> for Auth
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::One: CombineRejection<Rejection>,
{
    type Wrapped = AuthFilter<F>;

    fn wrap(&self, inner: F) -> Self::Wrapped {
        let config = self.config.clone();
        AuthFilter { config, inner }
    }
}

#[derive(Clone, Debug)]
struct Configured {
    authenticate_header: HeaderValue,
    authorizations: HashSet<Vec<u8>>,
}

fn split_at(bytes: &[u8], mid: usize) -> Option<(&[u8], &[u8])> {
    if mid <= bytes.len() {
        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `bytes`, which
        // fulfills the requirements of `from_raw_parts_mut`.
        Some(unsafe { (bytes.get_unchecked(..mid), bytes.get_unchecked(mid..)) })
    } else {
        None
    }
}

impl Configured {
    fn is_authorized(&self, headers: &http::HeaderMap) -> bool {
        const PREFIX: &'static [u8] = b"Basic ";

        let header = match headers.get(header::AUTHORIZATION) {
            Some(v) => v,
            None => return false,
        };

        let (prefix, rest) = match split_at(header.as_bytes(), PREFIX.len()) {
            Some((prefix, rest)) => (prefix, rest),
            None => return false,
        };

        prefix == PREFIX && self.authorizations.contains(rest)
    }
}

mod internal {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};

    use futures::{future, ready, TryFuture};
    use http::header;
    use pin_project::pin_project;

    use super::Configured;
    use crate::filter::{Filter, FilterBase, Internal, One};
    use crate::generic::Either;
    use crate::reject::IsReject;
    use crate::route;

    #[derive(Clone, Debug)]
    pub struct AuthFilter<F> {
        pub(super) config: Arc<Configured>,
        pub(super) inner: F,
    }

    impl<F> FilterBase for AuthFilter<F>
    where
        F: Filter,
        F::Extract: Send,
        F::Future: Future,
        F::Error: IsReject,
    {
        type Extract = One<Either<One<Unauthorized>, One<F::Extract>>>;
        type Error = F::Error;
        type Future = future::Either<
            future::Ready<Result<Self::Extract, Self::Error>>,
            WrappedFuture<F::Future>,
        >;

        fn filter(&self, _: Internal) -> Self::Future {
            let is_authorized = route::with(|route| self.config.is_authorized(route.headers()));

            if is_authorized {
                let wrapped = WrappedFuture {
                    inner: self.inner.filter(Internal),
                };
                future::Either::Right(wrapped)
            } else {
                let unauthorized = Unauthorized {
                    config: self.config.clone(),
                };
                future::Either::Left(future::ok((Either::A((unauthorized,)),)))
            }
        }
    }

    #[derive(Debug)]
    pub struct Unauthorized {
        config: Arc<Configured>,
    }

    impl crate::reply::Reply for Unauthorized {
        fn into_response(self) -> crate::reply::Response {
            let mut res = crate::reply::Response::default();
            *res.status_mut() = http::StatusCode::UNAUTHORIZED;
            res.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                self.config.authenticate_header.clone(),
            );
            res
        }
    }

    #[pin_project]
    #[derive(Debug)]
    pub struct WrappedFuture<F> {
        #[pin]
        inner: F,
    }

    impl<F> Future for WrappedFuture<F>
    where
        F: TryFuture,
        F::Error: IsReject,
    {
        type Output = Result<One<Either<One<Unauthorized>, One<F::Ok>>>, F::Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let pin = self.project();
            match ready!(pin.inner.try_poll(cx)) {
                Ok(inner) => {
                    let item = (inner,);
                    let item = (Either::B(item),);
                    Poll::Ready(Ok(item))
                }
                Err(err) => Poll::Ready(Err(err.into())),
            }
        }
    }
}
