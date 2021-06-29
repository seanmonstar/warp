//! Route information filter.
//!
//! Using `route` and `Info` from this module provides insights into the current
//! request that would otherwise be unobtainable.
use std::convert::Infallible;
use std::net::SocketAddr;
use std::ops::Deref;

use futures::future;
use http::{self, header};

use crate::filter::{filter_fn_one, Filter, One};
use crate::route::Route;

/// Create a `Filter` that extracts information about the current `Route`.
pub fn route() -> impl Filter<Extract = One<Info>, Error = Infallible> + Clone {
    filter_fn_one(|route| future::ok::<_, Infallible>(route.deref().into()))
}

/// Information about the current `Route`.
#[derive(Clone, Debug)]
pub struct Info {
    remote_addr: Option<SocketAddr>,
    method: http::Method,
    path: String,
    version: http::Version,
    headers: http::HeaderMap,
    uri: http::Uri,
}

impl From<&Route> for Info {
    fn from(route: &Route) -> Self {
        Self {
            remote_addr: route.remote_addr(),
            method: route.method().clone(),
            path: route.full_path().to_string(),
            version: route.version(),
            headers: route.headers().clone(),
            uri: route.uri().clone(),
        }
    }
}

impl Info {
    /// View the remote `SocketAddr` of the request.
    pub fn remote_addr(&self) -> &Option<SocketAddr> {
        &self.remote_addr
    }

    /// View the `http::Method` of the request.
    pub fn method(&self) -> &http::Method {
        &self.method
    }

    /// View the full URI of the request.
    pub fn uri(&self) -> &http::Uri {
        &self.uri
    }

    /// View the URI path of the request.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// View the `http::Version` of the request.
    pub fn version(&self) -> http::Version {
        self.version
    }

    /// View the referer of the request.
    pub fn referer(&self) -> Option<&str> {
        self.headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
    }

    /// View the user agent of the request.
    pub fn user_agent(&self) -> Option<&str> {
        self.headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
    }

    /// View the host of the request
    pub fn host(&self) -> Option<&str> {
        self.headers()
            .get(header::HOST)
            .and_then(|v| v.to_str().ok())
    }

    /// View the request headers.
    pub fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
}
