//! Path Filters
//!
//! The filters here work on the "path" of requests.
//!
//! - [`path`](./fn.path.html) matches a specific segment, like `/foo`.
//! - [`param`](./fn.param.html) tries to parse a segment into a type, like `/:u16`.
//! - [`end`](./fn.end.html) matches when the path end is found.
//! - [`path!`](../../macro.path.html) eases combining multiple `path` and `param` filters.
//!
//! # Routing
//!
//! Routing in warp is simple yet powerful.
//!
//! First up, matching a single segment:
//!
//! ```
//! use warp::Filter;
//!
//! // GET /hi
//! let hi = warp::path("hi").map(|| {
//!     "Hello, World!"
//! });
//! ```
//!
//! How about multiple segments? It's easiest with the `path!` macro:
//!
//! ```
//! # #[macro_use] extern crate warp; fn main() {
//! # use warp::Filter;
//! // GET /hello/from/warp
//! let hello_from_warp = path!("hello" / "from" / "warp").map(|| {
//!     "Hello from warp!"
//! });
//! # }
//! ```
//!
//! Neat! But do I handle **parameters** in paths?
//!
//! ```
//! # #[macro_use] extern crate warp; fn main() {
//! # use warp::Filter;
//! // GET /sum/:u32/:u32
//! let sum = path!("sum" / u32 / u32).map(|a, b| {
//!     format!("{} + {} = {}", a, b, a + b)
//! });
//! # }
//! ```
//!
//! In fact, any type that implements `FromStr` can be used, in any order:
//!
//! ```
//! # #[macro_use] extern crate warp; fn main() {
//! # use warp::Filter;
//! // GET /:u16/times/:u16
//! let times = path!(u16 / "times" / u16).map(|a, b| {
//!     format!("{} times {} = {}", a, b, a * b)
//! });
//! # }
//! ```
//!
//! Oh shoot, those math routes should be **mounted** at a different path,
//! is that possible? Yep!
//!
//! ```
//! # use warp::Filter;
//! # let sum = warp::any().map(warp::reply);
//! # let times = sum.clone();
//! // GET /math/sum/:u32/:u32
//! // GET /math/:u16/times/:u16
//! let math = warp::path("math");
//! let math_sum = math.and(sum);
//! let math_times = math.and(times);
//! ```
//!
//! What! `and`? What's that do?
//!
//! It combines the filters in a sort of "this and then that" order. In fact,
//! it's exactly what the `path!` macro has been doing internally.
//!
//! ```
//! # use warp::Filter;
//! // GET /bye/:string
//! let bye = warp::path("bye")
//!     .and(warp::path::param())
//!     .map(|name: String| {
//!         format!("Good bye, {}!", name)
//!     });
//! ```
//!
//! Ah, so, can filters do things besides `and`?
//!
//! Why, yes they can! They can also `or`! As you might expect, `or` creates a
//! "this or else that" chain of filters. If the first doesn't succeed, then
//! it tries the other.
//!
//! So, those `math` routes could have been **mounted** all as one, with `or`.
//!
//!
//! ```
//! # use warp::Filter;
//! # let sum = warp::any().map(warp::reply);
//! # let times = sum.clone();
//! // GET /math/sum/:u32/:u32
//! // GET /math/:u16/times/:u16
//! let math = warp::path("math")
//!     .and(sum.or(times));
//! ```
//!
//! It turns out, using `or` is how you combine everything together into a
//! single API.
//!
//! ```
//! # use warp::Filter;
//! # let hi = warp::any().map(warp::reply);
//! # let hello_from_warp = hi.clone();
//! # let bye = hi.clone();
//! # let math = hi.clone();
//! // GET /hi
//! // GET /hello/from/warp
//! // GET /bye/:string
//! // GET /math/sum/:u32/:u32
//! // GET /math/:u16/times/:u16
//! let routes = hi
//!     .or(hello_from_warp)
//!     .or(bye)
//!     .or(math);
//! ```

use std::fmt;
use std::str::FromStr;

use http::uri::PathAndQuery;

use filter::{filter_fn, one, Filter, One, Tuple};
use never::Never;
use reject::{self, Rejection};
use route::Route;

/// Create an exact match path segment `Filter`.
///
/// This will try to match exactly to the current request path segment.
///
/// # Panics
///
/// Exact path filters cannot be empty, or contain slashes.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Matches '/hello'
/// let hello = warp::path("hello")
///     .map(|| "Hello, World!");
/// ```
pub fn path(p: &'static str) -> impl Filter<Extract = (), Error = Rejection> + Copy {
    assert!(!p.is_empty(), "exact path segments should not be empty");
    assert!(
        !p.contains('/'),
        "exact path segments should not contain a slash: {:?}",
        p
    );

    segment(move |seg| {
        trace!("{:?}?: {:?}", p, seg);
        if seg == p {
            Ok(())
        } else {
            Err(reject::not_found())
        }
    })
}

#[doc(hidden)]
#[deprecated(note = "renamed to warp::path::end")]
pub fn index() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    end()
}

/// Matches the end of a route.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// // Matches '/'
/// let hello = warp::path::end()
///     .map(|| "Hello, World!");
/// ```
pub fn end() -> impl Filter<Extract = (), Error = Rejection> + Copy {
    filter_fn(move |route| {
        if route.path().is_empty() {
            Ok(())
        } else {
            Err(reject::not_found())
        }
    })
}

/// Extract a parameter from a path segment.
///
/// This will try to parse a value from the current request path
/// segment, and if successful, the value is returned as the `Filter`'s
/// "extracted" value.
///
/// If the value could not be parsed, rejects with a `404 Not Found`.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::path::param()
///     .map(|id: u32| {
///         format!("You asked for /{}", id)
///     });
/// ```
pub fn param<T: FromStr + Send>() -> impl Filter<Extract = One<T>, Error = Rejection> + Copy {
    segment(|seg| {
        trace!("param?: {:?}", seg);
        if seg.is_empty() {
            return Err(reject::not_found());
        }
        T::from_str(seg).map(one).map_err(|_| reject::not_found())
    })
}

/// Extract a parameter from a path segment.
///
/// This will try to parse a value from the current request path
/// segment, and if successful, the value is returned as the `Filter`'s
/// "extracted" value.
///
/// If the value could not be parsed, rejects with a `404 Not Found`. In
/// contrast of `param` method, it reports an error cause in response.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::path::param2()
///     .map(|id: u32| {
///         format!("You asked for /{}", id)
///     });
/// ```
pub fn param2<T>() -> impl Filter<Extract = One<T>, Error = Rejection> + Copy
where
    T: FromStr + Send,
    T::Err: Into<::reject::Cause>,
{
    segment(|seg| {
        trace!("param?: {:?}", seg);
        if seg.is_empty() {
            return Err(reject::not_found());
        }
        T::from_str(seg).map(one).map_err(|err| {
            #[allow(deprecated)]
            reject::not_found().with(err.into())
        })
    })
}

/// Extract the unmatched tail of the path.
///
/// This will return a `Tail`, which allows access to the rest of the path
/// that previous filters have not already matched.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::path("foo")
///     .and(warp::path::tail())
///     .map(|tail| {
///         // GET /foo/bar/baz would return "bar/baz".
///         format!("The tail after foo is {:?}", tail)
///     });
/// ```
pub fn tail() -> impl Filter<Extract = One<Tail>, Error = Never> + Copy {
    filter_fn(move |route| {
        let path = path_and_query(&route);
        let idx = route.matched_path_index();

        // Giving the user the full tail means we assume the full path
        // has been matched now.
        let end = path.path().len() - idx;
        route.set_unmatched_path(end);

        Ok(one(Tail {
            path,
            start_index: idx,
        }))
    })
}

/// Represents that tail part of a request path, returned by the `tail()` filter.
pub struct Tail {
    path: PathAndQuery,
    start_index: usize,
}

impl Tail {
    /// Get the `&str` representation of the remaining path.
    pub fn as_str(&self) -> &str {
        &self.path.path()[self.start_index..]
    }
}

impl fmt::Debug for Tail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// Peek at the unmatched tail of the path, without affecting the matched path.
///
/// This will return a `Peek`, which allows access to the rest of the path
/// that previous filters have not already matched. This differs from `tail`
/// in that `peek` will **not** set the entire path as matched.
///
/// # Example
///
/// ```
/// use warp::Filter;
///
/// let route = warp::path("foo")
///     .and(warp::path::peek())
///     .map(|peek| {
///         // GET /foo/bar/baz would return "bar/baz".
///         format!("The path after foo is {:?}", peek)
///     });
/// ```
pub fn peek() -> impl Filter<Extract = One<Peek>, Error = Never> + Copy {
    filter_fn(move |route| {
        let path = path_and_query(&route);
        let idx = route.matched_path_index();

        Ok(one(Peek {
            path,
            start_index: idx,
        }))
    })
}

/// Represents that tail part of a request path, returned by the `tail()` filter.
pub struct Peek {
    path: PathAndQuery,
    start_index: usize,
}

impl Peek {
    /// Get the `&str` representation of the remaining path.
    pub fn as_str(&self) -> &str {
        &self.path.path()[self.start_index..]
    }

    /// Get an iterator over the segments of the peeked path.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.as_str().split('/').filter(|seg| !seg.is_empty())
    }
}

impl fmt::Debug for Peek {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// Returns the full request path, irrespective of other filters.
///
/// This will return a `FullPath`, which can be stringified to return the
/// full path of the request.
///
/// This is more useful in generic pre/post-processing filters, and should
/// probably not be used for request matching/routing.
///
/// # Example
///
/// ```
/// use warp::{Filter, path::FullPath};
/// use std::{collections::HashMap, sync::{Arc, Mutex}};
///
/// let counts = Arc::new(Mutex::new(HashMap::new()));
/// let access_counter = warp::path::full()
///     .map(move |path: FullPath| {
///         let mut counts = counts.lock().unwrap();
///
///         *counts.entry(path.as_str().to_string())
///             .and_modify(|c| *c += 1)
///             .or_insert(0)
///     });
///
/// let route = warp::path("foo")
///     .and(warp::path("bar"))
///     .and(access_counter)
///     .map(|count| {
///         format!("This is the {}th visit to this URL!", count)
///     });
/// ```
pub fn full() -> impl Filter<Extract = One<FullPath>, Error = Never> + Copy {
    filter_fn(move |route| Ok(one(FullPath(path_and_query(&route)))))
}

/// Represents the full request path, returned by the `full()` filter.
pub struct FullPath(PathAndQuery);

impl FullPath {
    /// Get the `&str` representation of the request path.
    pub fn as_str(&self) -> &str {
        &self.0.path()
    }
}

impl fmt::Debug for FullPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

fn segment<F, U>(func: F) -> impl Filter<Extract = U, Error = Rejection> + Copy
where
    F: Fn(&str) -> Result<U, Rejection> + Copy,
    U: Tuple + Send,
{
    filter_fn(move |route| {
        let (u, idx) = {
            let seg = route
                .path()
                .splitn(2, '/')
                .next()
                .expect("split always has at least 1");
            (func(seg)?, seg.len())
        };
        route.set_unmatched_path(idx);
        Ok(u)
    })
}

fn path_and_query(route: &Route) -> PathAndQuery {
    route
        .uri()
        .path_and_query()
        .expect("server URIs should always have path_and_query")
        .clone()
}

/// Convenient way to chain multiple path filters together.
///
/// Any number of either type identifiers or string expressions can be passed,
/// each separated by a forward slash (`/`). Strings will be used to match
/// path segments exactly, and type identifiers are used just like
/// [`param`](filters::path::param) filters.
///
/// # Example
///
/// ```
/// # #[macro_use] extern crate warp; fn main() {
/// use warp::Filter;
///
/// // Match `/sum/:a/:b`
/// let route = path!("sum" / u32 / u32)
///     .map(|a, b| {
///         format!("{} + {} = {}", a, b, a + b)
///     });
/// # }
/// ```
///
/// The equivalent filter chain without using the `path!` macro looks this:
///
/// ```
/// use warp::Filter;
///
/// let route = warp::path("sum")
///     .and(warp::path::param::<u32>())
///     .and(warp::path::param::<u32>())
///     .map(|a, b| {
///         format!("{} + {} = {}", a, b, a + b)
///     });
/// ```
///
/// In fact, this is exactly what the macro expands to.
#[macro_export]
macro_rules! path {
    (@start $first:tt $(/ $tail:tt)*) => ({
        let __p = path!(@segment $first);
        $(
        let __p = $crate::Filter::and(__p, path!(@segment $tail));
        )*
        __p
    });
    (@segment $param:ty) => (
        $crate::path::param::<$param>()
    );
    (@segment $s:expr) => (
        $crate::path($s)
    );
    ($($pieces:tt)*) => (
        path!(@start $($pieces)*)
    );
}
