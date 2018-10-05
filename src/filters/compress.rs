//! compress things !

use ::filter::{Filter, Map, WrapSealed};
use ::reply::Reply;

use self::sealed::WithGzip_;

/// compress things !
pub fn gzip() -> WithGzip {
    WithGzip
}

/// Wrap a `Filter` to always set a header.
#[derive(Clone, Debug)]
pub struct WithGzip;

impl<F, R> WrapSealed<F> for WithGzip
where
    F: Filter<Extract=(R,)>,
    R: Reply,
{
    type Wrapped = Map<F, WithGzip_>;

    fn wrap(&self, filter: F) -> Self::Wrapped {
        let with = WithGzip_ { with: self.clone() };
        filter.map(with)
    }
}

mod sealed {
    use std::io;
    use http::Response;
    use hyper::Body;

    use flate2::Compression;
    use flate2::read::GzEncoder;
    use futures::{Stream, Future};

    use ::generic::{Func, One};
    use ::reply::{Reply, Reply_};

    use super::WithGzip;

    #[derive(Clone)]
    #[allow(missing_debug_implementations)]
    pub struct WithGzip_ {
        pub(super) with: WithGzip,
    }

    impl<R: Reply> Func<One<R>> for WithGzip_ {
        type Output = Reply_;

        fn call(&self, args: One<R>) -> Self::Output {
            let (mut parts, body) = args.0.into_response().into_parts();

            // TODO: this is really not asynchronous...
            let stream = body.concat2().wait().unwrap();
            let mut encoder = GzEncoder::new(&stream[..], Compression::fast());

            let mut buff = Vec::new();
            let len = io::copy(&mut encoder, &mut buff).unwrap();

            let body = Body::from(buff);

            // TODO: set the content-length,
            //       but how do we know the final size asynchronously ?
            //       do we need to return "content-encoding: chunked" along with each chunk size ?
            parts.headers.insert("content-length", len.to_string().parse().unwrap());

            // TODO: what do we do if the request doesn't have "accept-encoding: gzip" ?
            // Append the "gzip" header at the end if it already exists...
            parts.headers.append("content-encoding", "gzip".parse().unwrap());

            Reply_(Response::from_parts(parts, body))
        }
    }
}
