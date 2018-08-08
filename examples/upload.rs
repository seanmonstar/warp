#![deny(warnings)]
extern crate bytes;
extern crate multipart;
extern crate pretty_env_logger;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate warp;

use bytes::Buf;
use multipart::mock::StdoutTee;
use multipart::server::Multipart;
use multipart::server::save::Entries;
use multipart::server::save::SaveResult::*;
use regex::Regex;
use std::io::{self, Cursor, Write};
use std::str;
use warp::Filter;

///
/// curl -v -F "FileUpload=@Cargo.toml" http://127.0.0.1:3030/upload
///

fn main() {
    pretty_env_logger::init();

    warp::serve(
        warp::post(
            warp::path("upload").and(warp::path::index())
                .and(warp::header::<String>("Content-Type"))
                .and(warp::body::concat())
                .and_then(recieve_upload)
        )
    ).run(([127, 0, 0, 1], 3030));
}

fn recieve_upload(content_type: String, body: warp::body::FullBody) -> Result<String, warp::Rejection> {
    let boundary_re = Regex::new(r"^multipart/form-data;\sboundary=(?P<boundary>.+)$").unwrap();

    let boundary = match boundary_re.captures(&content_type) {
        Some(caps) => {
            caps.name("boundary").unwrap().as_str()
        }
        None => {
            return Err(warp::reject::bad_request());
        }
    };

    match process_upload(boundary, body.bytes().to_vec()) {
        Ok(_resp) => {
            return Ok("Upload processed...\n".into());
        }
        Err(_err) => return Err(warp::reject::bad_request())
    };
}

fn process_upload(boundary: &str, data: Vec<u8>) -> Result<Vec<u8>, warp::Rejection> {
    let mut out = Vec::new();

    match Multipart::with_body(Cursor::new(data), boundary).save().temp() {
        Full(entries) => match process_entries(entries, &mut out) {
            Ok(entry) => entry,
            Err(_) => return Err(warp::reject::bad_request())
        },
        Partial(partial, reason) => {
            writeln!(out, "Request partially processed: {:?}", reason).unwrap();
            if let Some(field) = partial.partial {
                writeln!(out, "Stopped on field: {:?}", field.source.headers).unwrap();
            };
            process_entries(partial.entries, &mut out).unwrap();
        }
        Error(_e) => return Err(warp::reject::bad_request()),
    }

    Ok(out)
}

fn process_entries(entries: Entries, mut out: &mut Vec<u8>) -> io::Result<()> {
    {
        let stdout = io::stdout();
        let tee = StdoutTee::new(&mut out, &stdout);
        entries.write_debug(tee)?;
    }

    writeln!(out, "Entries processed")
}
