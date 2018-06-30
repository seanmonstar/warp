use std::cell::{Cell, RefCell};

use http;

use ::Request;
use ::reply::WarpBody;

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
    body: RefCell<WarpBody>,

    segments_index: Cell<usize>,
    segments_total: usize,
}


impl Route {
    pub(crate) fn new(req: Request) -> Route {
        let cnt = req
            .uri()
            .path()
            .split('/')
            // -1 because the before the first slash is skipped
            .count() - 1;
        let (parts, body) = req.into_parts();
        let req = http::Request::from_parts(parts, ());
        Route {
            req,
            body: RefCell::new(body),
            segments_index: Cell::new(0),
            segments_total: cnt,
        }
    }

    pub(crate) fn method(&self) -> &http::Method {
        self.req.method()
    }

    pub(crate) fn headers(&self) -> &http::HeaderMap {
        self.req.headers()
    }

    pub(crate) fn query(&self) -> Option<&str> {
        self.req.uri().query()
    }

    pub(crate) fn has_more_segments(&self) -> bool {
        self.segments_index.get() != self.segments_total
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

    pub(crate) fn filter_segment<F, U>(&self, fun: F) -> Option<U>
    where
        F: FnOnce(&str) -> Option<U>,
    {
        if self.segments_index.get() == self.segments_total {
            None
        } else {
            fun(
                self
                    .req
                    .uri()
                    .path()
                    //TODO: record this on Route::init
                    .split('/')
                    .skip(self.segments_index.get() + 1)
                    .next()
                    .expect("Route segment unimplemented")
            )
                .map(|val| {
                    let idx = self.segments_index.get();
                    self.segments_index.set(idx + 1);
                    val
                })
        }
    }

    pub(crate) fn take_body(&self) -> Option<WarpBody> {
        if self.segments_index.get() == self.segments_total {
            let body = self.body.borrow_mut().route_take();
            Some(body)
        } else {
            trace!("route segments not fully matched, cannot take body");
            None
        }
    }

    pub(crate) fn into_req(self) -> Request {
        let body = self.body.into_inner();
        self.req.map(move |()| body)
    }
}

