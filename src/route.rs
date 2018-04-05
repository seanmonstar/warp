use http;

use ::Request;

#[derive(Debug)]
pub struct Route<'a> {
    req: &'a Request,

    segments_index: usize,
    segments_total: usize,
}

impl<'a> Route<'a> {
    pub(crate) fn new(req: &'a Request) -> Route<'a> {
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
            let seg = self
                .req
                .uri()
                .path()
                //TODO: record this on Route::init
                .split('/')
                .skip(self.segments_index + 1)
                .next()
                .expect("Route segment unimplemented");
            fun(seg).map(|val| {
                self.segments_index += 1;
                (self, val)
            })
        }
    }
}

impl<'a> Clone for Route<'a> {
    fn clone(&self) -> Route<'a> {
        Route {
            req: self.req,
            segments_index: self.segments_index,
            segments_total: self.segments_total,
        }
    }
}
