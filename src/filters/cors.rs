//! CORS Filters

use std::collections::HashSet;
use std::sync::Arc;

use headers::{AccessControlAllowHeaders, AccessControlAllowMethods, HeaderMapExt};
use http::{
    self,
    header::{self, HeaderName, HeaderValue},
    HttpTryFrom,
};

use filter::{Filter, WrapSealed};
use reject::{CombineRejection, Rejection};
use reply::Reply;

use self::internal::{CorsFilter, IntoOrigin, Seconds};

/// Create a wrapping filter that exposes [CORS][] behavior for a wrapped
/// filter.
///
/// [CORS]: https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let cors = warp::cors()
///     .allow_origin("https://hyper.rs")
///     .allow_methods(vec!["GET", "POST", "DELETE"]);
///
/// let route = warp::any()
///     .map(warp::reply)
///     .with(cors);
/// ```
pub fn cors() -> Cors {
    Cors {
        credentials: false,
        headers: HashSet::new(),
        max_age: None,
        methods: HashSet::new(),
        origins: None,
    }
}

/// A wrapping filter constructed via `warp::cors()`.
#[derive(Clone, Debug)]
pub struct Cors {
    credentials: bool,
    headers: HashSet<HeaderName>,
    max_age: Option<u64>,
    methods: HashSet<http::Method>,
    origins: Option<HashSet<HeaderValue>>,
}

impl Cors {
    /// Sets whether to add the `Access-Control-Allow-Credentials` header.
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.credentials = allow;
        self
    }

    /// Adds a method to the existing list of allowed request methods.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::Method`.
    pub fn allow_method<M>(mut self, method: M) -> Self
    where
        http::Method: HttpTryFrom<M>,
    {
        let method = match HttpTryFrom::try_from(method) {
            Ok(m) => m,
            Err(_) => panic!("illegal Method"),
        };
        self.methods.insert(method);
        self
    }

    /// Adds multiple methods to the existing list of allowed request methods.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::Method`.
    pub fn allow_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator,
        http::Method: HttpTryFrom<I::Item>,
    {
        let iter = methods.into_iter().map(|m| match HttpTryFrom::try_from(m) {
            Ok(m) => m,
            Err(_) => panic!("illegal Method"),
        });
        self.methods.extend(iter);
        self
    }

    /// Adds a header to the list of allowed request headers.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `http::header::HeaderName`.
    pub fn allow_header<H>(mut self, header: H) -> Self
    where
        HeaderName: HttpTryFrom<H>,
    {
        let header = match HttpTryFrom::try_from(header) {
            Ok(m) => m,
            Err(_) => panic!("illegal Header"),
        };
        self.headers.insert(header);
        self
    }

    /// Adds multiple headers to the list of allowed request headers.
    ///
    /// # Panics
    ///
    /// Panics if any of the headers are not a valid `http::header::HeaderName`.
    pub fn allow_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator,
        HeaderName: HttpTryFrom<I::Item>,
    {
        let iter = headers.into_iter().map(|h| match HttpTryFrom::try_from(h) {
            Ok(h) => h,
            Err(_) => panic!("illegal Method"),
        });
        self.headers.extend(iter);
        self
    }

    /// Sets that *any* `Origin` header is allowed.
    ///
    /// # Warning
    ///
    /// This can allow websites you didn't instead to access this resource,
    /// it is usually better to set an explicit list.
    pub fn allow_any_origin(mut self) -> Self {
        self.origins = None;
        self
    }

    /// Add an origin to the existing list of allowed `Origin`s.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `Origin`.
    pub fn allow_origin(self, origin: impl IntoOrigin) -> Self {
        self.allow_origins(Some(origin))
    }

    /// Add multiple origins to the existing list of allowed `Origin`s.
    ///
    /// # Panics
    ///
    /// Panics if the provided argument is not a valid `Origin`.
    pub fn allow_origins<I>(mut self, origins: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoOrigin,
    {
        let iter = origins
            .into_iter()
            .map(IntoOrigin::into_origin)
            .map(|origin| {
                origin
                    .to_string()
                    .parse()
                    .expect("Origin is always a valid HeaderValue")
            });

        self.origins.get_or_insert_with(HashSet::new).extend(iter);

        self
    }

    /// Sets the `Access-Control-Max-Age` header.
    ///
    /// # Example
    ///
    ///
    /// ```
    /// use std::time::Duration;
    /// use warp::Filter;
    ///
    /// let cors = warp::cors()
    ///     .max_age(30) // 30u32 seconds
    ///     .max_age(Duration::from_secs(30)); // or a Duration
    /// ```
    pub fn max_age(mut self, seconds: impl Seconds) -> Self {
        self.max_age = Some(seconds.seconds());
        self
    }
}

impl<F> WrapSealed<F> for Cors
where
    F: Filter + Clone + Send + Sync + 'static,
    F::Extract: Reply,
    F::Error: CombineRejection<Rejection>,
    <F::Error as CombineRejection<Rejection>>::Rejection: CombineRejection<Rejection>,
{
    type Wrapped = CorsFilter<F>;

    fn wrap(&self, inner: F) -> Self::Wrapped {
        let config = Arc::new(Configured {
            cors: self.clone(),
            headers_header: self.headers.iter().map(|m| m.clone()).collect(),
            methods_header: self.methods.iter().map(|m| m.clone()).collect(),
        });

        CorsFilter { config, inner }
    }
}

/// An error used to reject requests that are forbidden by a `cors` filter.
#[derive(Debug)]
pub struct CorsForbidden {
    kind: Forbidden,
}

#[derive(Debug)]
enum Forbidden {
    OriginNotAllowed,
    MethodNotAllowed,
    HeaderNotAllowed,
}

impl ::std::fmt::Display for CorsForbidden {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let detail = match self.kind {
            Forbidden::OriginNotAllowed => "origin not allowed",
            Forbidden::MethodNotAllowed => "request-method not allowed",
            Forbidden::HeaderNotAllowed => "header not allowed",
        };
        write!(f, "CORS request forbidden: {}", detail)
    }
}

impl ::std::error::Error for CorsForbidden {
    fn description(&self) -> &str {
        "CORS request forbidden"
    }
}

#[derive(Clone, Debug)]
struct Configured {
    cors: Cors,
    headers_header: AccessControlAllowHeaders,
    methods_header: AccessControlAllowMethods,
}

enum Validated {
    Preflight(HeaderValue),
    Simple(HeaderValue),
    NotCors,
}

impl Configured {
    fn check_request(
        &self,
        method: &http::Method,
        headers: &http::HeaderMap,
    ) -> Result<Validated, Forbidden> {
        match (headers.get(header::ORIGIN), method) {
            (Some(origin), &http::Method::OPTIONS) => {
                // OPTIONS requests are preflight CORS requests...

                if !self.is_origin_allowed(origin) {
                    return Err(Forbidden::OriginNotAllowed);
                }

                if let Some(req_method) = headers.get(header::ACCESS_CONTROL_REQUEST_METHOD) {
                    if !self.is_method_allowed(req_method) {
                        return Err(Forbidden::MethodNotAllowed);
                    }
                } else {
                    trace!("preflight request missing access-control-request-method header");
                    return Err(Forbidden::MethodNotAllowed);
                }

                if let Some(req_headers) = headers.get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
                    let headers = req_headers
                        .to_str()
                        .map_err(|_| Forbidden::HeaderNotAllowed)?;
                    for header in headers.split(",") {
                        if !self.is_header_allowed(header) {
                            return Err(Forbidden::HeaderNotAllowed);
                        }
                    }
                }

                Ok(Validated::Preflight(origin.clone()))
            }
            (Some(origin), _) => {
                // Any other method, simply check for a valid origin...

                trace!("origin header: {:?}", origin);
                if self.is_origin_allowed(origin) {
                    Ok(Validated::Simple(origin.clone()))
                } else {
                    Err(Forbidden::OriginNotAllowed)
                }
            }
            (None, _) => {
                // No `ORIGIN` header means this isn't CORS!
                Ok(Validated::NotCors)
            }
        }
    }

    fn is_method_allowed(&self, header: &HeaderValue) -> bool {
        http::Method::from_bytes(header.as_bytes())
            .map(|method| self.cors.methods.contains(&method))
            .unwrap_or(false)
    }

    fn is_header_allowed(&self, header: &str) -> bool {
        HeaderName::from_bytes(header.as_bytes())
            .map(|header| self.cors.headers.contains(&header))
            .unwrap_or(false)
    }

    fn is_origin_allowed(&self, origin: &HeaderValue) -> bool {
        if let Some(ref allowed) = self.cors.origins {
            allowed.contains(origin)
        } else {
            true
        }
    }

    fn append_preflight_headers(&self, headers: &mut http::HeaderMap) {
        self.append_common_headers(headers);

        headers.typed_insert(self.headers_header.clone());
        headers.typed_insert(self.methods_header.clone());

        if let Some(max_age) = self.cors.max_age {
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, max_age.into());
        }
    }

    fn append_common_headers(&self, headers: &mut http::HeaderMap) {
        if self.cors.credentials {
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }
    }
}

mod internal {
    use std::sync::Arc;

    use futures::{future, Future, Poll};
    use headers::Origin;
    use http::header;

    use super::{Configured, CorsForbidden, Validated};
    use filter::{Filter, FilterBase, One};
    use generic::Either;
    use reject::{CombineRejection, Rejection};
    use route;

    #[derive(Debug)]
    pub struct CorsFilter<F> {
        pub(super) config: Arc<Configured>,
        pub(super) inner: F,
    }

    impl<F> FilterBase for CorsFilter<F>
    where
        F: Filter,
        F::Extract: Send,
        F::Error: CombineRejection<Rejection>,
    {
        type Extract =
            One<Either<One<Preflight>, One<Either<One<Wrapped<F::Extract>>, F::Extract>>>>;
        type Error = <F::Error as CombineRejection<Rejection>>::Rejection;
        type Future = future::Either<
            future::FutureResult<Self::Extract, Self::Error>,
            WrappedFuture<F::Future>,
        >;

        fn filter(&self) -> Self::Future {
            let validated =
                route::with(|route| self.config.check_request(route.method(), route.headers()));

            match validated {
                Ok(Validated::Preflight(origin)) => {
                    let preflight = Preflight {
                        config: self.config.clone(),
                        origin,
                    };
                    future::Either::A(future::ok((Either::A((preflight,)),)))
                }
                Ok(Validated::Simple(origin)) => future::Either::B(WrappedFuture {
                    inner: self.inner.filter(),
                    wrapped: Some((self.config.clone(), origin)),
                }),
                Ok(Validated::NotCors) => future::Either::B(WrappedFuture {
                    inner: self.inner.filter(),
                    wrapped: None,
                }),
                Err(err) => {
                    let rejection = ::reject::known(CorsForbidden { kind: err });
                    future::Either::A(future::err(rejection.into()))
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct Preflight {
        config: Arc<Configured>,
        origin: header::HeaderValue,
    }

    impl ::reply::ReplySealed for Preflight {
        fn into_response(self) -> ::reply::Response {
            let mut res = ::reply::Response::default();
            self.config.append_preflight_headers(res.headers_mut());
            res.headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, self.origin);
            res
        }
    }

    #[derive(Debug)]
    pub struct Wrapped<R> {
        config: Arc<Configured>,
        inner: R,
        origin: header::HeaderValue,
    }

    impl<R> ::reply::ReplySealed for Wrapped<R>
    where
        R: ::reply::ReplySealed,
    {
        fn into_response(self) -> ::reply::Response {
            let mut res = self.inner.into_response();
            self.config.append_common_headers(res.headers_mut());
            res.headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, self.origin);
            res
        }
    }

    #[derive(Debug)]
    pub struct WrappedFuture<F> {
        inner: F,
        wrapped: Option<(Arc<Configured>, header::HeaderValue)>,
    }

    impl<F> Future for WrappedFuture<F>
    where
        F: Future,
        F::Error: CombineRejection<Rejection>,
    {
        type Item = One<Either<One<Preflight>, One<Either<One<Wrapped<F::Item>>, F::Item>>>>;
        type Error = <F::Error as CombineRejection<Rejection>>::Rejection;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let inner = try_ready!(self.inner.poll());
            let item = if let Some((config, origin)) = self.wrapped.take() {
                (Either::A((Wrapped {
                    config,
                    inner,
                    origin,
                },)),)
            } else {
                (Either::B(inner),)
            };
            let item = (Either::B(item),);
            Ok(item.into())
        }
    }

    pub trait Seconds {
        fn seconds(self) -> u64;
    }

    impl Seconds for u32 {
        fn seconds(self) -> u64 {
            self.into()
        }
    }

    impl Seconds for ::std::time::Duration {
        fn seconds(self) -> u64 {
            self.as_secs()
        }
    }

    pub trait IntoOrigin {
        fn into_origin(self) -> Origin;
    }

    impl<'a> IntoOrigin for &'a str {
        fn into_origin(self) -> Origin {
            let mut parts = self.splitn(2, "://");
            let scheme = parts.next().expect("missing scheme");
            let rest = parts.next().expect("missing scheme");

            Origin::try_from_parts(scheme, rest, None).expect("invalid Origin")
        }
    }
}
