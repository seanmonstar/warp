#![deny(warnings)]
extern crate futures;
extern crate pretty_env_logger;
extern crate warp;

use futures::{Future, Stream};
use warp::Filter;

#[test]
fn form_fields() {
    let _ = pretty_env_logger::try_init();

    let route = warp::multipart::form()
        .and_then(|form: warp::multipart::FormData| {
            // Collect the fields into (name, value): (String, Vec<u8>)
            form
                .and_then(|part| {
                    let name = part.name().to_string();
                    part.concat2().map(move |value| (name, value))
                })
                .collect()
                .map_err(|e| -> warp::Rejection {
                    panic!("multipart error: {:?}", e);
                })
        });

    let boundary = "--abcdef1234--";
    let body = format!("\
        --{0}\r\n\
        content-disposition: form-data; name=\"foo\"\r\n\r\n\
        bar\r\n\
        --{0}--\r\n\
    ", boundary);

    let req = warp::test::request()
        .method("POST")
        .header("content-length", body.len())
        .header("content-type", format!("multipart/form-data; boundary={}", boundary))
        .body(body);

    let vec = req.filter(&route).unwrap();
    assert_eq!(&vec[0].0, "foo");
    assert_eq!(&vec[0].1, b"bar");
}
