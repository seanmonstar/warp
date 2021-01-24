#![deny(warnings)]
use num_cpus;
use warp::Filter;

const BODY: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="UTF-8" />
    <title>Hello benchmark</title>
  </head>
  <body>
    This is a static content to check the warp performance.
  </body>
</html>"#;

fn main() -> std::io::Result<()> {
    let route = warp::any().map(move || BODY);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()?
        .block_on(async {
            warp::serve(route).run(([0, 0, 0, 0], 3030)).await;
        });
    Ok(())
}
