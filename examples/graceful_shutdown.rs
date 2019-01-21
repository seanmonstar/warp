#![deny(warnings)]

#[macro_use]
extern crate log;

extern crate futures;
extern crate pretty_env_logger;
extern crate signal_hook;
extern crate tokio;
extern crate warp;

use futures::future::Future;
use futures::stream::Stream;
use signal_hook::iterator::Signals;
use warp::Filter;

fn main() {
    pretty_env_logger::init();
    let routes = warp::any().map(|| {
        std::thread::sleep(std::time::Duration::from_secs(5));
        "slept for 5 seconds\n"
    });

    let shutdown = Signals::new(&[signal_hook::SIGINT, signal_hook::SIGTERM])
        .expect("successful signal registration")
        .into_async()
        .expect("successful async conversion")
        .into_future()
        .map(|_sig| info!("shutdown requested"))
        .map_err(|e| panic!("signal error {}", e.0));

    let port = 3030;
    info!("now serving on http://127.0.0.1:{}/", port);
    let (_addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], port), shutdown);
    tokio::run(server);
    info!("clean shutdown completed");
}
