#![deny(warnings)]

use std::num::NonZeroU16;

use serde_derive::Serialize;
use warp::http::StatusCode;
use warp::reply::Response;
use warp::{Filter, Reply};

#[tokio::main]
async fn main() {
    let math = warp::path!("math" / u16)
        .and(warp::header::<u16>("div-by"))
        .map(div_by);

    let routes = warp::get().and(math);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

// Reply is implemented for Result<impl Reply, impl Reply>.
// This makes it easy to handle errors since bad requests are just mapped to Result::Err().
fn div_by(num: u16, denom: u16) -> Result<impl Reply, DivideByZero> {
    let denom = NonZeroU16::new(denom).ok_or(DivideByZero)?;

    Ok(warp::reply::json(&Math {
        op: format!("{} / {}", num, denom),
        output: num / denom.get(),
    }))
}

// Error

#[derive(Debug)]
struct DivideByZero;

// We have to implement Reply for our error types.
impl Reply for DivideByZero {
    fn into_response(self) -> Response {
        let code = StatusCode::BAD_REQUEST;
        let message = "DIVIDE_BY_ZERO";
        let json = warp::reply::json(&ErrorMessage {
            code: code.as_u16(),
            message: message.into(),
        });

        warp::reply::with_status(json, code).into_response()
    }
}

// JSON replies

/// A successful math operation.
#[derive(Serialize)]
struct Math {
    op: String,
    output: u16,
}

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}
