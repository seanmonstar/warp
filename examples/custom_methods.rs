#![deny(warnings)]
use std::net::SocketAddr;
use std::str::FromStr;

use warp::hyper::StatusCode;
use warp::{hyper::Method, reject, Filter, Rejection, Reply};

#[derive(Debug)]
struct MethodError;
impl reject::Reject for MethodError {}

fn method(name: &str) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    let method =
        Method::from_str(name).expect(&format!("Method name {} could not be converted", name));

    warp::method()
        .and_then(move |m: Method| {
            let method = method.clone();
            async move {
                if m == method {
                    Ok(())
                } else {
                    Err(reject::custom(MethodError))
                }
            }
        })
        .untuple_one()
}

pub async fn handle_not_found(reject: Rejection) -> Result<impl Reply, Rejection> {
    if reject.is_not_found() {
        Ok(StatusCode::NOT_FOUND)
    } else {
        Err(reject)
    }
}

pub async fn handle_custom(reject: Rejection) -> Result<impl Reply, Rejection> {
    if reject.find::<MethodError>().is_some() {
        Ok(StatusCode::METHOD_NOT_ALLOWED)
    } else {
        Err(reject)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = "[::]:3030".parse()?;

    let foo_route = method("FOO")
        .and(warp::path!("foo"))
        .map(|| "Success")
        .recover(handle_not_found);

    let bar_route = method("BAR")
        .and(warp::path!("bar"))
        .map(|| "Success")
        .recover(handle_not_found);

    warp::serve(foo_route.or(bar_route).recover(handle_custom))
        .run(address)
        .await;

    Ok(())
}
