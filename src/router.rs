use std::net::SocketAddr;

use http::{self, Request, Response};
use hyper::Body;

use ::filter::{And, Filter, FilterResult};
use ::filter::method::{method, Method};
use ::handler::{IntoHandler, Handler};
use ::reply::{Reply, WarpBody};
use ::server::{serve, WarpService};

pub fn router() -> Router<NotFound> {
    Router {
        routes: NotFound(()),
    }
}

pub struct Router<R> {
    routes: R,
}

impl<R> Router<R>
where
    R: Route + 'static,
{
    pub fn route<F, H>(self, filter: F, handler: H)
    -> Router<
        Routes<
            FilteredRoute<
                F,
                H::Handler,
            >,
            R,
        >,
    >
    where
        F: Filter,
        F::Extract: 'static,
        H: IntoHandler<F::Extract>,
        H::Handler: 'static,
    {
        let r = FilteredRoute {
            filter: filter,
            handler: handler.into_handler(),
        };
        let routes = assert_route(Routes {
            left: r,
            right: self.routes,
        });

        Router {
            routes,
        }
    }

    /*
    pub fn get<F, H>(self, filter: F, handler: H)
    -> Router<
        Routes<
            FilteredRoute<
                And<
                    Method,
                    F,
                >,
                H::Handler,
            >,
            R,
        >,
    >
    where
        F: Filter,
        F::Extract: 'static,
        H: IntoHandler<F::Extract>,
        H::Handler: 'static,
    {
        let m = method(http::Method::GET);
        self.route(m.and(filter), handler)
        /*
        let h = handler.into_handler();
        self.route(m.and(filter), move |(), e| {
            h.handle(e)
        })
        */
    }
*/

    pub fn run<A>(self, addr: A)
    where
        A: Into<SocketAddr>,
    {
        serve(self).run(addr)
    }
}

fn assert_route<T: Route>(r: T) -> T {
    r
}

impl<R> WarpService for Router<R>
where
    R: Route,
{
    type Reply = Response<WarpBody>;

    fn call(&self, req: Request<WarpBody>) -> Self::Reply {
        match self.routes.handle(req) {
            RouteResult::Replied(rep) => rep,
            RouteResult::NoMatch(_req) => {
                unimplemented!("route not handled!");
            }
        }
    }
}

pub trait Route {
    fn handle(&self, req: Request<WarpBody>) -> RouteResult;
}

pub enum RouteResult {
    Replied(Response<WarpBody>),
    NoMatch(Request<WarpBody>),
}

pub struct NotFound(());

impl Route for NotFound {
    fn handle(&self, _req: Request<WarpBody>) -> RouteResult {
        RouteResult::Replied(Response::builder()
            .status(404)
            .header("content-length", "0")
            .body(WarpBody(Body::empty()))
            .unwrap())
    }
}

pub struct FilteredRoute<F, H> {
    filter: F,
    handler: H,
}

impl<F, H> Route for FilteredRoute<F, H>
where
    F: Filter,
    H: Handler<F::Extract>,
{
    fn handle(&self, mut req: Request<WarpBody>) -> RouteResult {
        match self.filter.filter(&mut req) {
            FilterResult::Matched(extracted) => {
                RouteResult::Replied(self.handler.handle(extracted).into_response())
            },
            FilterResult::Skipped => RouteResult::NoMatch(req),
        }
    }
}

pub struct Routes<T, U> {
    left: T,
    right: U,
}

impl<T, U> Route for Routes<T, U>
where
    T: Route,
    U: Route,
{
    fn handle(&self, req: Request<WarpBody>) -> RouteResult {
        match self.left.handle(req) {
            RouteResult::Replied(rep) => RouteResult::Replied(rep),
            RouteResult::NoMatch(req) => {
                self.right.handle(req)
            }
        }
    }
}
