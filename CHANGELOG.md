### v0.1.5 (October 3, 2018)

- **Features**:
  - Serve `index.html` automatically with `warp::fs::dir` filter.
  - Include `last-modified` header with `warp::fs` filters.
  - Add `warp::redirect` to easily reply with redirections.
  - Add `warp::reply::{with_status, with_header}` to wrap `impl Reply`s directly with a new status code or header.
  - Add support for running a warp `Server` with a custom source of incoming connections.
    - `Server::run_incoming` to have the runtime started automatically.
    - `Server::serve_incoming` to get a future to run on existing runtime.
    - These can be used to support Unix Domain Sockets, TLS, and other transports.
  - Add `Rejection::into_cause()` to retrieve the original error of a rejection back.
  - Add `Rejection::json()` to convert a rejection into a JSON response.

- **Fixes**
  - Internal errors in warp that result in rendering a `500 Internal Server Error` are now also logged at the `error` level.


### v0.1.4 (September 25, 2018)

- **Features**:
  - Add `warp::reply::with::headers(HeaderMap)` filter wrapper.
  - Add `warp::cookie::optional()` to get an optional cookie value.
  - Add `warp::path::full()` to be able to extract the full request path without affecting route matching.
  - Add graceful shutdown support to the `Server`.
  - Allow empty query strings to be treated as for `warp::query()`.

### v0.1.3 (August 28, 2018)

- **Features**:
  - Add `warp::reject::forbidden()` to represent `403 Forbidden` responses.
  - Add `Rejection::with(cause)` to customize rejection messages.
- **Fixes**:
  - Fix `warp::body::form` to allow charsets in the `content-type` header.

### v0.1.2 (August 14, 2018)

- **Features**:
  - Implemented `Reply` for `Response<impl Into<hyper::Body>`, allowing streaming response bodies.
  - Add `warp::body::stream()` filter to access the request body as an `impl Stream`.
  - Add `warp::ws2()` as a more flexible websocket filter.
    - This allows passing other extracted values to the upgrade callback, such as a value from a header or path.
    - Deprecates `warp::ws()`, and `ws2()` will become `ws()` in 0.2.
  - Add `warp::get2()`, `warp::post2()`, `warp::put2()`, and `warp::delete2()` as more standard method filters that are used via chaining instead of nesting.
    - `get()`, `post()`, `put()`, and `delete()` are deprecated, and the new versions will become them in 0.2.
  - Add `Filter::unify()` for when a filter returns `Either<T, T>`, converting the `Either` into the inner `T`, regardless of which variant it was.
    - This requires that both sides of the `Either` be the same type.
    - This can be useful when extracting a value that might be present in different places of the request.
      
      ```rust
      // Allow `MyId` to be a path parameter or a header...
      let id = warp::path::param::<MyId>()
          .or(warp::header::<MyId>())
          .unify();
      
      // A way of providing default values...
      let dnt = warp::header::<bool>("dnt")
          .or(warp::any().map(|| true))
          .unify();
      ```
  - Add `content-type` header automatically to replies from `file` and `dir` filters based on file extension.
  - Add `warp::head()`, `warp::options()`, and `warp::patch()` as new Method filters.
  - Try to use OS blocksize in `warp::fs` filters.
- **Fixes**:
  - Chaining filters that try to consume the request body will log that the body is already consumed, and return a `500 Internal Server Error` rejection.

### v0.1.1 (August 7, 2018)

- **Features**:
  - Add `warp::query::raw()` filter to get query as a `String`.
  - Add `Filter::recover()` to ease customizing of rejected responses.
  - Add `warp::header::headers_clone()` filter to get a clone of request's `HeaderMap`.
  - Add `warp::path::tail()` filter to get remaining "tail" of the request path.
- **Fixes**:
  - URL decode path segments in `warp::fs` filters.


## v0.1.0 (August 1, 2018)

- Intial release.
