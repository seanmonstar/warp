use scoped_tls::scoped_thread_local;
use std::cell::RefCell;
use std::mem;
use std::net::SocketAddr;
use std::str::FromStr;

use http;
use http::uri::Authority;
use hyper::Body;

use crate::Request;

scoped_thread_local!(static ROUTE: RefCell<Route>);

pub(crate) fn set<F, U>(r: &RefCell<Route>, func: F) -> U
where
    F: FnOnce() -> U,
{
    ROUTE.set(r, func)
}

pub(crate) fn is_set() -> bool {
    ROUTE.is_set()
}

pub(crate) fn with<F, R>(func: F) -> R
where
    F: FnOnce(&mut Route) -> R,
{
    ROUTE.with(move |route| func(&mut *route.borrow_mut()))
}

#[derive(Debug)]
pub(crate) struct Route {
    body: BodyState,
    remote_addr: Option<SocketAddr>,
    req: Request,
    segments_index: usize,
}

#[derive(Debug)]
enum BodyState {
    Ready,
    Taken,
}

#[derive(Debug)]
pub(crate) struct HostError;

impl Route {
    pub(crate) fn new(req: Request, remote_addr: Option<SocketAddr>) -> RefCell<Route> {
        let segments_index = if req.uri().path().starts_with('/') {
            // Skip the beginning slash.
            1
        } else {
            0
        };

        RefCell::new(Route {
            body: BodyState::Ready,
            remote_addr,
            req,
            segments_index,
        })
    }

    pub(crate) fn method(&self) -> &http::Method {
        self.req.method()
    }

    pub(crate) fn headers(&self) -> &http::HeaderMap {
        self.req.headers()
    }

    pub(crate) fn version(&self) -> http::Version {
        self.req.version()
    }

    pub(crate) fn extensions(&self) -> &http::Extensions {
        self.req.extensions()
    }

    #[cfg(feature = "websocket")]
    pub(crate) fn extensions_mut(&mut self) -> &mut http::Extensions {
        self.req.extensions_mut()
    }

    pub(crate) fn uri(&self) -> &http::Uri {
        self.req.uri()
    }

    pub(crate) fn path(&self) -> &str {
        &self.req.uri().path()[self.segments_index..]
    }

    pub(crate) fn full_path(&self) -> &str {
        self.req.uri().path()
    }

    pub(crate) fn set_unmatched_path(&mut self, index: usize) {
        let index = self.segments_index + index;
        let path = self.req.uri().path();
        if path.is_empty() {
            // malformed path
            return;
        } else if path.len() == index {
            self.segments_index = index;
        } else {
            debug_assert_eq!(path.as_bytes()[index], b'/');
            self.segments_index = index + 1;
        }
    }

    pub(crate) fn query(&self) -> Option<&str> {
        self.req.uri().query()
    }

    pub(crate) fn matched_path_index(&self) -> usize {
        self.segments_index
    }

    pub(crate) fn reset_matched_path_index(&mut self, index: usize) {
        debug_assert!(
            index <= self.segments_index,
            "reset_match_path_index should not be bigger: current={}, arg={}",
            self.segments_index,
            index,
        );
        self.segments_index = index;
    }

    pub(crate) fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    pub(crate) fn take_body(&mut self) -> Option<Body> {
        match self.body {
            BodyState::Ready => {
                let body = mem::replace(self.req.body_mut(), Body::empty());
                self.body = BodyState::Taken;
                Some(body)
            }
            BodyState::Taken => None,
        }
    }

    /// The authority can be sent by clients in various ways:
    ///
    ///  1) in the "target URI"
    ///    a) serialized in the start line (HTTP/1.1 proxy requests)
    ///    b) serialized in `:authority` pseudo-header (HTTP/2 generated - "SHOULD")
    ///  2) in the `Host` header (HTTP/1.1 origin requests, HTTP/2 converted)
    ///
    /// Hyper transparently handles 1a/1b, but not 2, so we must look at both.
    pub(crate) fn host(&self) -> Result<Option<Authority>, HostError> {
        let from_uri = self.req.uri().authority();
        let from_header = self.req.headers().get(http::header::HOST).map(|value|
                // Header present, parse it
                value.to_str().map_err(|_| HostError)
                    .and_then(|value| Authority::from_str(value).map_err(|_| HostError)));

        match (from_uri, from_header) {
            // no authority in the request (HTTP/1.0 or non-conforming)
            (None, None) => Ok(None),

            // authority specified in either or both matching
            (Some(a), None) => Ok(Some(a.clone())),
            (None, Some(Ok(a))) => Ok(Some(a)),
            (Some(a), Some(Ok(b))) if *a == b => Ok(Some(b)),

            // mismatch
            (Some(_), Some(Ok(_))) => Err(HostError),

            // parse error
            (_, Some(Err(r))) => Err(r),
        }
    }
}
