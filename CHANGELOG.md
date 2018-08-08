### v0.1.1 (August 7, 2018)

- **Features**:
  - add `warp::query::raw()` filter to get query as a `String`.
  - add `Filter::recover()` to ease customizing of rejected responses.
  - add `warp::header::headers_clone()` filter to get a clone of request's `HeaderMap`.
  - add `warp::path::tail()` filter to get remaining "tail" of the request path.
- **Fixes**:
  - URL decode path segments in `warp::fs` filters.


## v0.1.0 (August 1, 2018)

- Intial release.
