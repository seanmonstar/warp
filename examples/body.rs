extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate pretty_env_logger;
extern crate warp;

use warp::{Filter, Future};

#[derive(Deserialize, Serialize)]
struct Employee {
    name: String,
    rate: u32,
}

fn main() {
    pretty_env_logger::init();

    let promote = warp::path::exact("employees")
        .unit_and(warp::path::<u32>())
        .and(warp::body::json::<Employee>())
        .map(|(rate, json)| {
            // json is a future
            json.map(move |mut employee| {
                employee.rate = rate;
                warp::reply::json(employee)
            })
            .or_else(|_| Ok(warp::reply::client_error()))
        });

    // POST /employees/:rate  {"name":"Sean","rate":2}
    let routes = warp::post(promote);

    warp::serve(routes.service())
        .run(([127, 0, 0, 1], 3030));
}
