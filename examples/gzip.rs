extern crate flate2;
extern crate warp;

use flate2::Compression;
use flate2::write::GzEncoder;

use std::io::Write;

use warp::{Filter, Rejection, Reply};
use warp::http::{Response, StatusCode};

const INDEX_HTML: &str = r#"
<html>
    <head>
        <title>GZIP compression example</title>
    </head>
    <body>
        <h1 style="text-align: center;margin-top: 50px;width: 100%;">GZIP activated!</h1>
    </body>
</html>
"#;

macro_rules! return_gzip_err {
    ($content:expr, $typ:expr, $err:expr) => {{
        eprintln!("Error in gzip compression: {}", $err);
        return Response::builder()
                        .header("content-type", $typ)
                        .body($content.replace("GZIP activated",
                                               "GZIP **not** activated (because of an error)")
                                      .as_bytes()
                                      .to_owned())
    }}
}

macro_rules! return_gzip_or_not {
    ($encoding:expr, $content:expr, $typ:expr) => {{
         let s = $encoding.to_lowercase();
         if s.contains("gzip") || s.contains("*") {
             let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
             if gz.write_all($content.as_bytes()).is_err() {
                return_gzip_err!($content, $typ, "write_all failed")
             }
             if let Ok(buffer) = gz.finish() {
                Response::builder()
                         .header("content-type", $typ)
                         .header("content-encoding", "gzip")
                         .body(buffer.to_owned())
             } else {
                return_gzip_err!($content, $typ, "finish failed")
             }
         } else {
             Response::builder()
                      .header("content-type", $typ)
                      .body($content.replace("GZIP activated", "GZIP **not** activated")
                                    .as_bytes()
                                    .to_owned())
         }
    }}
}

fn customize_error(err: Rejection) -> Result<impl Reply, Rejection> {
    match err.status() {
        StatusCode::NOT_FOUND => {
            Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body("you get a 404, and *you* get a 404..."))
        },
        StatusCode::INTERNAL_SERVER_ERROR => {
            Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(":fire: this is fine"))
        }
        _ => {
            Err(err)
        }
    }
}

fn main() {
    let index = warp::index()
                     .and(warp::header::<String>("accept-encoding"))
                     .map(|encoding: String| {
        return_gzip_or_not!(encoding, INDEX_HTML, "text/html")
    });

    let routes = warp::get2().and(index).recover(customize_error);
    println!("Starting server on '127.0.0.1:4321'");
    warp::serve(routes).run(([127, 0, 0, 1], 4321));
}