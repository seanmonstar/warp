use http;

use ::Request;
use ::reply::WarpBody;

#[derive(Debug)]
pub struct Route<'a> {
    req: &'a mut Request,

    segments_index: usize,
    segments_total: usize,
}

impl<'a> Route<'a> {
    pub(crate) fn new(req: &'a mut Request) -> Route<'a> {
        let cnt = req
            .uri()
            .path()
            .split('/')
            // -1 because the before the first slash is skipped
            .count() - 1;
        Route {
            req,
            segments_index: 0,
            segments_total: cnt,
        }
    }

    pub fn method(&self) -> &http::Method {
        self.req.method()
    }

    pub fn uri_mut(&mut self) -> &mut http::Uri {
        self.req.uri_mut()
    }

    pub fn uri(&self) -> &http::Uri {
        self.req.uri()
    }

    pub fn headers(&self) -> &http::HeaderMap {
        self.req.headers()
    }

    pub(crate) fn has_more_segments(&self) -> bool {
        self.segments_index != self.segments_total
    }

    pub(crate) fn filter_segment<F, U>(mut self, fun: F) -> Option<(Self, U)>
    where
        F: FnOnce(&str) -> Option<U>,
    {
        if self.segments_index == self.segments_total {
            None
        } else {
            fun(
                self
                    .req
                    .uri()
                    .path()
                    //TODO: record this on Route::init
                    .split('/')
                    .skip(self.segments_index + 1)
                    .next()
                    .expect("Route segment unimplemented")
            )
                .map(|val| {
                    self.segments_index += 1;
                    (self, val)
                })
        }
    }

    pub(crate) fn take_body(self) -> Option<(Self, WarpBody)> {
        if self.segments_index == self.segments_total {
            let body = self.req.body_mut().route_take();
            Some((self, body))
        } else {
            None
        }
    }

    pub(crate) fn scoped<F, U>(self, fun: F) -> (Self, Option<U>)
    where
        F: FnOnce(Route) -> Option<(Route, U)>,
    {
        // Woah! What's going on here!
        //
        // TODO: make sure this is actually safe.
        //
        // The idea is that we need to give a sub-scoped Route to a closure,
        // which *may* return a mutated Route, or might drop it. If they return
        // a new one, we drop our original. If they dropped the sub-scope, we
        // assume it's gone, and return our original.
        unsafe {
            let sub = Route {
                req: ::std::mem::transmute::<_, &mut Request>(self.req as *mut _),
                segments_index: self.segments_index,
                segments_total: self.segments_total,
            };

            if let Some((sub, val)) = fun(sub) {
                (sub, Some(val))
            } else {
                (self, None)
            }
        }
    }
}

