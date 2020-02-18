# Examples

Welcome to the examples! These show off `warp`'s functionality and explain how to use it.

## Getting Started

- [`hello.rs`](./hello.rs) - Just a basic "Hello World" API
- [`routing.rs`](./routing.rs) - Builds up a more complex set of routes and shows how to combine filters
- [`body.rs`](./body.rs) - What's a good API without parsing data from the request body?
- [`headers.rs`](./headers.rs) - Parsing data from the request headers
- [`rejections.rs`](./rejections.rs) - Your APIs are obviously perfect, but for silly others who call them incorrectly you'll want to define errors for them
- [`futures.rs`](./futures.rs) - Wait, wait! ... Or how to integrate futures into filters
- [`todos.rs`](./todos.rs) - Putting this all together with a proper app

## Further Use Cases

### Serving HTML and Other Files

- [`file.rs`](./file.rs) - Serving static files
- [`dir.rs`](./dir.rs) - Or a whole directory of files
- [`handlebars_template.rs`](./handlebars_template.rs) - Using Handlebars to fill in an HTML template

### Websockets

Hooray! `warp` also includes built-in support for WebSockets

- [`websockets.rs`](./websockets.rs) - Basic handling of a WebSocket upgrade
- [`websockets_chat.rs`](./websockets_chat.rs) - Full WebSocket app

### Server-Side Events

- [`sse.rs`](./sse.rs) - Basic Server-Side Event
- [`sse_chat.rs`](./sse_chat.rs) - Full SSE app

### TLS

- [`tls.rs`](./tls.rs) - can i haz security?

### Autoreloading

- [`autoreload.rs`](./autoreload.rs) - Change some code and watch the server reload automatically!
