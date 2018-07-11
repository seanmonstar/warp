use std::cell::RefCell;
use std::mem;

use http;
use hyper::Body;

use ::Request;

task_local! {
    static ROUTE: RefCell<Route> = RefCell::new(Route::new(Request::default()))
}

pub(crate) fn set(req: Request) {
    ROUTE.with(move |route| {
        *route.borrow_mut() = Route::new(req);
    });
}

pub(crate) fn with<F, R>(func: F) -> R
where
    F: Fn(&mut Route) -> R,
{
    ROUTE.with(move |route| {
        func(&mut *route
            .borrow_mut())
    })
}

// Must be public because used in FilterBase::filter(route).
//
// But both FilterBase and Route aren't exposed outside of the crate.
// Ideally the missing_docs lint helps prove this.
#[derive(Debug)]
pub struct Route {
    req: Request,
    segments_index: usize,
}


impl Route {
    pub(crate) fn new(req: Request) -> Route {
        debug_assert_eq!(req.uri().path().as_bytes()[0], b'/');

        Route {
            req,
            // always start at 1, since paths are `/...`.
            segments_index: 1,
        }
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

    pub(crate) fn path(&self) -> &str {
        &self.req.uri().path()[self.segments_index..]
    }

    pub(crate) fn full_path(&self) -> &str {
        self.req.uri().path()
    }

    pub(crate) fn set_unmatched_path(&mut self, index: usize) {
        let index = self.segments_index + index;

        let path = self.req.uri().path();

        if path.len() == index {
            self.segments_index = index;
        } else {
            debug_assert_eq!(
                path.as_bytes()[index],
                b'/',
            );

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

    pub(crate) fn take_body(&mut self) -> Body {
        mem::replace(self.req.body_mut(), Body::empty())
    }
}

