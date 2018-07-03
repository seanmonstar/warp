use std::cell::{Cell, RefCell};
use std::mem;

use http;
use hyper::Body;

use ::Request;

scoped_thread_local!(static ROUTE: Route);

pub(crate) fn set<F, R>(route: &Route, f: F) -> R
where
    F: FnOnce() -> R
{
    ROUTE.set(route, f)
}

pub(crate) fn with<F, R>(f: F) -> R
where
    F: FnMut(&Route) -> R,
{
    ROUTE.with(f)
}

pub(crate) fn is_set() -> bool {
    ROUTE.is_set()
}

#[derive(Debug)]
pub(crate) struct Route {
    req: http::Request<()>,
    body: RefCell<Body>,

    segments_index: Cell<usize>,
}


impl Route {
    pub(crate) fn new(req: Request) -> Route {
        debug_assert_eq!(req.uri().path().as_bytes()[0], b'/');

        let (parts, body) = req.into_parts();
        let req = http::Request::from_parts(parts, ());
        Route {
            req,
            body: RefCell::new(body),
            // always start at 1, since paths are `/...`.
            segments_index: Cell::new(1),
        }
    }

    pub(crate) fn method(&self) -> &http::Method {
        self.req.method()
    }

    pub(crate) fn headers(&self) -> &http::HeaderMap {
        self.req.headers()
    }

    pub(crate) fn path(&self) -> &str {
        &self.req.uri().path()[self.segments_index.get()..]
    }

    pub(crate) fn set_unmatched_path(&self, index: usize) {
        let index = self.segments_index.get() + index;

        let path = self.req.uri().path();

        if path.len() == index {
            self.segments_index.set(index);
        } else {
            debug_assert_eq!(
                path.as_bytes()[index],
                b'/',
            );

            self.segments_index.set(index + 1);
        }
    }

    pub(crate) fn query(&self) -> Option<&str> {
        self.req.uri().query()
    }

    pub(crate) fn has_more_segments(&self) -> bool {
        self.segments_index.get() < self.req.uri().path().len()
    }

    pub(crate) fn transaction<F, R>(&self, op: F) -> Option<R>
    where
        F: FnOnce() -> Option<R>
    {
        let idx = self.segments_index.get();
        match op() {
            None => {
                self.segments_index.set(idx);
                None
            },
            some => some,
        }
    }

    pub(crate) fn take_body(&self) -> Body {
        mem::replace(&mut *self.body.borrow_mut(), Body::empty())
    }
}

